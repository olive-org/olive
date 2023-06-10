//! Process Scheduler Context

use std::collections::{hash_map::Iter, HashMap};

use crate::bpmn::schema::{ExtensionElements, Item, Properties, TaskDefinition, TaskHeaders};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    service_type: Option<String>,
    headers: HashMap<String, Value>,
    properties: HashMap<String, Value>,
}

impl Context {
    /// Creates new Context
    pub fn new() -> Self {
        Self {
            service_type: None,
            headers: HashMap::new(),
            properties: HashMap::new(),
        }
    }

    pub fn set_service_type(&mut self, service_type: Option<String>) {
        self.service_type = service_type
    }

    pub fn get_service_type(&self) -> Option<&String> {
        self.service_type.as_ref()
    }

    pub fn get_service_type_mut(&mut self) -> Option<&mut String> {
        self.service_type.as_mut()
    }

    pub fn get_header(&self, name: &str) -> Option<&Value> {
        self.headers.get(name)
    }

    pub fn get_headers(&self) -> Iter<String, Value> {
        self.headers.iter()
    }

    pub fn get_header_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.headers.get_mut(name)
    }

    pub fn set_header(&mut self, name: &str, value: &Value) {
        self.headers.insert(name.to_string(), value.clone());
    }

    pub fn get_property(&self, name: &str) -> Option<&Value> {
        self.properties.get(name)
    }

    pub fn get_property_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.properties.get_mut(name)
    }

    pub fn get_properties(&self) -> Iter<String, Value> {
        self.properties.iter()
    }

    pub fn set_property(&mut self, name: &str, value: &Value) {
        self.properties.insert(name.to_string(), value.clone());
    }

    pub fn bond(&mut self, other: &Self) {
        for (k, v) in &mut self.headers {
            let result = match v {
                Value::String(x) if x.is_empty() => other.get_header(k.as_str()),
                Value::Object(x) if x.is_empty() => other.get_header(k.as_str()),
                _ => None,
            };

            if let Some(value) = result {
                *v = value.clone()
            }
        }
        for (k, v) in &mut self.properties {
            let result = match v {
                Value::String(x) if x.is_empty() => other.get_property(k.as_str()),
                Value::Object(x) if x.is_empty() => other.get_property(k.as_str()),
                _ => None,
            };

            if let Some(value) = result {
                *v = value.clone()
            }
        }
    }

    pub fn merge(&mut self, other: &Self) {
        if self.service_type.is_none() {
            self.service_type = other.service_type.clone();
        }
        for (k, v) in &other.headers {
            match self.get_header_mut(k.as_str()) {
                None => self.set_header(k, v),
                Some(Value::Null) => self.set_header(k, v),
                _ => {}
            }
        }
        for (k, v) in &other.properties {
            match self.get_property_mut(k.as_str()) {
                None => self.set_property(k, v),
                Some(Value::Null) => self.set_property(k, v),
                _ => {}
            }
        }
    }
}

impl From<ExtensionElements> for Context {
    fn from(element: ExtensionElements) -> Self {
        let properties = match element.properties {
            Some(p) => p
                .items
                .iter()
                .filter_map(|item| parse_item(item))
                .collect::<HashMap<String, Value>>(),
            None => HashMap::new(),
        };
        let headers = match element.task_headers {
            Some(p) => p
                .items
                .iter()
                .filter_map(|item| parse_item(item))
                .collect::<HashMap<String, Value>>(),
            None => HashMap::new(),
        };
        let service_type = match element.task_definition {
            Some(d) => d.typ,
            None => None,
        };

        Self {
            service_type,
            headers,
            properties,
        }
    }
}

impl Into<ExtensionElements> for Context {
    fn into(self) -> ExtensionElements {
        let task_definition = Some(TaskDefinition {
            typ: self.service_type,
        });

        let task_headers = Some(TaskHeaders {
            items: self
                .headers
                .iter()
                .map(|(name, value)| parse_context_item((name, value)))
                .collect(),
        });

        let properties = Some(Properties {
            items: self
                .properties
                .iter()
                .map(|(name, value)| parse_context_item((name, value)))
                .collect(),
        });

        ExtensionElements {
            task_definition,
            task_headers,
            properties,
            ..Default::default()
        }
    }
}

pub fn parse_item(item: &Item) -> Option<(String, Value)> {
    if item.value.is_none() || item.key.is_none() {
        return None;
    }

    let value = item.value.as_ref().unwrap().as_str();
    if let Some(name) = &item.key {
        let mut serde_value = Value::Null;
        if let Some(typ) = &item.typ {
            match typ.as_str() {
                "integer" => {
                    if let Ok(v) = value.parse::<i64>() {
                        serde_value = Value::Number(v.into());
                    }
                }
                "float" => {
                    if let Ok(f) = value.parse::<f64>() {
                        serde_value = json!(f);
                    }
                }
                "boolean" | "bool" => {
                    if let Ok(ok) = value.parse::<bool>() {
                        serde_value = json!(ok);
                    }
                }
                "object" => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Map<String, Value>>(
                        &html_escape::decode_html_entities(value),
                    ) {
                        serde_value = Value::Object(v)
                    }
                }
                _ => serde_value = Value::String(value.to_string()),
            }
        }
        return Some((name.to_string(), serde_value));
    }
    None
}

pub fn parse_context_item((name, value): (&String, &Value)) -> Item {
    let typ = match value {
        Value::Bool(_) => "boolean",
        Value::Number(f) => match f.as_f64() {
            Some(_) => "float",
            None => "integer",
        },
        Value::Object(_) => "object",
        _ => "string",
    };
    Item {
        key: Some(name.to_string()),
        value: Some(value.to_string()),
        typ: Some(typ.to_string()),
    }
}

#[cfg(test)]
mod test {
    use crate::bpmn::schema::ExtensionElements;
    use serde_json::json;

    use super::Context;

    #[test]
    fn new_context() {
        let mut ctx = Context::new();
        ctx.set_header("a", &json!("a"));
        assert_eq!(ctx.get_header("a"), Some(&json!("a")));
    }

    #[test]
    fn context_to_extension_element() {
        let mut ctx = Context::new();
        ctx.set_header("a", &json!("a"));
        ctx.set_service_type(Some("service".to_string()));
        let el = ExtensionElements::from(ctx.into());
        assert_eq!(el.task_definition.unwrap().typ, Some("service".to_string()));
    }

    #[test]
    fn merge_context() {
        let mut c1 = Context::new();
        let mut c2 = Context::new();

        c1.set_service_type(Some("service".to_string()));
        c1.set_header("a", &json!("a".to_string()));
        c2.set_property("version", &json!(1));

        c2.merge(&c1);
        assert_eq!(c2.get_service_type(), Some(&"service".to_string()));
        assert_eq!(c2.get_header("a"), Some(&json!("a".to_string())));
        assert_eq!(c2.get_property("version"), Some(&json!(1)));
    }

    #[test]
    fn bound_context() {
        let mut c1 = Context::new();
        let mut c2 = Context::new();

        c1.set_header("a", &json!("".to_string()));
        c2.set_header("a", &json!("a".to_string()));
        c1.bond(&c2);

        assert_eq!(c1.get_header("a"), Some(&json!("a".to_string())));
    }

    #[test]
    fn escape_test() {
        let s = r#"{&#34;a&#34;: &#34;b&#34;}"#;
        let ss = html_escape::decode_html_entities(s);
        println!("{:?}", ss)
    }
}
