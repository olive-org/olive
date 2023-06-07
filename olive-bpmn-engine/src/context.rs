//! Process Scheduler Context

use std::collections::HashMap;

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

    pub fn get_header_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.headers.get_mut(name)
    }

    pub fn set_header(&mut self, name: &str, value: &Value) {
        self.headers.insert(name.to_string(), value.clone());
    }

    pub fn get_properties(&self, name: &str) -> Option<&Value> {
        self.properties.get(name)
    }

    pub fn get_properties_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.properties.get_mut(name)
    }

    pub fn set_properties(&mut self, name: &str, value: &Value) {
        self.properties.insert(name.to_string(), value.clone());
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
}
