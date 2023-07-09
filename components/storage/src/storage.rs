use std::{path::Path, sync::Arc};

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use olive_proto::{ProcessDefinitions, ProcessInstance};
use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, IteratorMode, Options, ReadOptions, TransactionDB,
    TransactionDBOptions,
};
use thiserror::Error;

const DEFINITIONS_PREFIX: &str = "definitions_";
const DEFINITIONS_CURRENT_VERSION: &str = "definition_version_";
const PROCESS_PREFIX: &str = "process_";

/// Process start error
#[derive(Error, Debug, PartialEq)]
pub enum StorageError {
    /// Key not found
    #[error("key not found")]
    ErrKeyNotFound,
    /// failed to deserialize
    #[error("error deserialize")]
    ErrDeserialize,
}

#[derive(Clone)]
pub struct Storage {
    db: Arc<TransactionDB>,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Arc<Storage>> {
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);

        let mut txn_opts = TransactionDBOptions::default();

        let definitions_cf = ColumnFamilyDescriptor::new("definitions", Options::default());
        let process_cf = ColumnFamilyDescriptor::new("process", Options::default());

        let db = TransactionDB::open_cf_descriptors(
            &db_opts,
            &txn_opts,
            db_path,
            vec![definitions_cf, process_cf],
        )?;

        let storage = Storage { db: Arc::new(db) };
        Ok(Arc::new(storage))
    }

    fn definitions_cf(&self) -> &ColumnFamily {
        self.db.cf_handle("definitions").unwrap()
    }

    fn get_current_version(&self, cf: &ColumnFamily, key: &str) -> Result<i64> {
        let value = self.db.get_cf(&cf, key)?;
        let version = value.context(anyhow!(StorageError::ErrKeyNotFound))?;
        let res = String::from_utf8(version)?;
        let version = res.parse::<i64>()?;
        Ok(version)
    }

    pub fn get_definitions(&self, id: &str, version: Option<i64>) -> Result<ProcessDefinitions> {
        let prefix = DEFINITIONS_PREFIX.to_string() + id;
        let cf = self.definitions_cf();

        let value = match version {
            Some(v) => {
                let key = prefix + "_" + v.to_string().as_str();
                self.db
                    .get_cf(cf, key)?
                    .context(anyhow!(StorageError::ErrKeyNotFound))?
            }
            None => {
                let mut read_opts = ReadOptions::default();
                read_opts.set_iterate_lower_bound(prefix);
                let mode = IteratorMode::End;
                let mut iter = self.db.iterator_cf_opt(cf, read_opts, mode);
                let (_, value) = iter
                    .next()
                    .context(anyhow!(StorageError::ErrKeyNotFound))?
                    .context(anyhow!(StorageError::ErrKeyNotFound))?;

                value.to_vec()
            }
        };

        serde_json::from_slice(&value).context(anyhow!(StorageError::ErrDeserialize))
    }

    pub fn get_definitions_vec(&self, id: &str) -> Result<Vec<ProcessDefinitions>> {
        let prefix = DEFINITIONS_PREFIX.to_string() + id + "_";
        let cf = self.definitions_cf();
        let mut read_opts = ReadOptions::default();
        read_opts.set_iterate_lower_bound(prefix);
        let mode = IteratorMode::End;
        let iter = self.db.iterator_cf_opt(cf, read_opts, mode);

        let vec = iter
            .filter_map(|item| item.ok())
            .flat_map(|(_, value)| {
                serde_json::from_slice(&value).context(anyhow!(StorageError::ErrDeserialize))
            })
            .collect();

        Ok(vec)
    }

    pub fn put_definitions(&self, definitions: &mut ProcessDefinitions) -> Result<i64> {
        let cf = self.definitions_cf();
        let version_key = DEFINITIONS_CURRENT_VERSION.to_string() + definitions.process_id.as_str();
        let current_version = self.get_current_version(cf, &version_key).unwrap_or(0);

        definitions.process_version = current_version + 1;
        definitions.creation_timestamp = Local::now().timestamp();
        let data = serde_json::to_vec(&definitions)?;

        let key = DEFINITIONS_PREFIX.to_string()
            + definitions.process_id.as_str()
            + "_"
            + definitions.process_version.to_string().as_str();
        let txn = self.db.transaction();
        txn.put_cf(cf, version_key, (current_version + 1).to_string())?;
        txn.put_cf(cf, key, data)?;
        txn.commit()?;

        Ok(definitions.process_version)
    }

    pub fn del_definitions(&self, id: &str, version: Option<i64>) -> Result<()> {
        let definitions = self.get_definitions(id, version)?;
        let cf = self.definitions_cf();
        let key = DEFINITIONS_PREFIX.to_string()
            + definitions.process_id.as_str()
            + "_"
            + definitions.process_version.to_string().as_str();
        self.db.delete_cf(cf, key)?;
        Ok(())
    }

    fn process_cf(&self) -> &ColumnFamily {
        self.db.cf_handle("process").unwrap()
    }

    pub fn get_process(&self, id: &str, key: &str) -> Result<ProcessInstance> {
        let key = PROCESS_PREFIX.to_string() + id + "_" + key;
        let cf = self.process_cf();
        let value = self
            .db
            .get_cf(cf, key)?
            .context(anyhow!(StorageError::ErrKeyNotFound))?;
        serde_json::from_slice(&value).context(anyhow!(StorageError::ErrDeserialize))
    }

    pub fn get_process_vec(&self, id: &str) -> Result<Vec<ProcessInstance>> {
        let prefix = PROCESS_PREFIX.to_string() + id + "_";
        let cf = self.process_cf();
        let mut read_opts = ReadOptions::default();
        read_opts.set_iterate_lower_bound(prefix);
        let mode = IteratorMode::End;
        let iter = self.db.iterator_cf_opt(cf, read_opts, mode);

        let vec = iter
            .filter_map(|item| item.ok())
            .flat_map(|(_, value)| {
                serde_json::from_slice(&value).context(anyhow!(StorageError::ErrDeserialize))
            })
            .collect();

        Ok(vec)
    }

    pub fn put_process(&self, process: &mut ProcessInstance) -> Result<()> {
        let cf = self.process_cf();

        let data = serde_json::to_vec(&process)?;

        let key = PROCESS_PREFIX.to_string()
            + process.process_id.as_str()
            + "_"
            + process.key.to_string().as_str();
        let txn = self.db.transaction();
        txn.put_cf(cf, key, data)?;

        Ok(())
    }

    pub fn del_process(&self, id: &str, key: &str) -> Result<()> {
        let process = self.get_process(id, key)?;
        let cf = self.process_cf();
        let key = PROCESS_PREFIX.to_string()
            + process.process_id.as_str()
            + "_"
            + process.key.to_string().as_str();
        self.db.delete_cf(cf, key)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        fs,
        time::{self, Instant, SystemTime},
    };

    use anyhow::Result;
    use olive_proto::ProcessDefinitions;

    use crate::Storage;

    #[test]
    fn test_new_storage() -> Result<()> {
        let dir = "data";
        let s = Storage::new(dir)?;

        let mut d = ProcessDefinitions {
            process_id: "test".to_string(),
            ..Default::default()
        };
        let v = s.put_definitions(&mut d)?;
        assert_eq!(v, 1);

        let dd = s.get_definitions("test", None)?;
        assert_eq!(d, dd);

        let version = s.put_definitions(&mut d)?;
        s.del_definitions("test", Some(version))?;
        println!("{:?}", s.get_definitions_vec("test")?);

        drop(s);
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
