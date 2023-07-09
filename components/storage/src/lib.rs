mod storage;
pub use storage::Storage;

#[cfg(test)]
mod tests {
    use anyhow::{Result, anyhow, Context};
    use rocksdb::{ColumnFamilyDescriptor, Options, DB, ReadOptions, IteratorMode};
    use std::{fs, time::{self, Instant}};

    #[test]
    fn test_rocksdb() {
        let mut db_opts = Options::default();
        db_opts.create_missing_column_families(true);
        db_opts.create_if_missing(true);
        let cf = ColumnFamilyDescriptor::new("cf", Options::default());

        let dir = "data";
        let db = DB::open_cf_descriptors(&db_opts, dir, vec![cf]).unwrap();

        if let Some(cf) = db.cf_handle("cf") {
            db.put_cf(&cf, b"key_2", b"").unwrap();
            db.put_cf(&cf, b"key_3", b"").unwrap();
            db.put_cf(&cf, b"key_1", b"").unwrap();
            db.put_cf(&cf, b"key_4", b"").unwrap();
            db.put_cf(&cf, b"ke_3", b"").unwrap();
            let _ = db.delete_cf(&cf, b"key_2");
            let mut read_opts = ReadOptions::default();
            read_opts.set_iterate_lower_bound("key_");
            let mode = IteratorMode::End;
            for v in db.iterator_cf_opt(&cf, read_opts, mode) {
                if let Ok((key, value)) = v {
                    println!("key={:?}, value={:?}", String::from_utf8(key.to_vec()), String::from_utf8(value.to_vec()))
                }
            }
        }
        let _ = DB::destroy(&Options::default(), dir);
        let _ = fs::remove_dir_all(dir);
    }


    #[test]
    fn test_convert() -> Result<()> {
        let v: Option<i32> = None;
        let _ss = v.context(anyhow!("aa"))?;

        Ok(())
    }
}
