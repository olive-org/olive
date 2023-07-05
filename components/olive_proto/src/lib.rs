pub mod api {
    tonic::include_proto!("api");
}
pub use api::*;

pub mod server {
    tonic::include_proto!("server");
}
pub use server::*;

#[cfg(test)]
mod tests {
    use super::api;
    use super::server;
    use serde_json;

    #[test]
    fn test_proto_build() {
        let node = api::FlowNode { key: 2 };
        assert_eq!(node.key, 2);
        let req = server::DeployDefinitionsRequest {
            name: "test".to_string(),
            definitions: "".to_string(),
        };
        assert_eq!(req.name, "test".to_string())
    }

    #[test]
    fn test_proto_serde() {
        let node = api::FlowNode { key: 2 };
        let v = serde_json::to_string(&node);
        assert_eq!(v.unwrap(), r#"{"key":2}"#.to_string());
    }
}
