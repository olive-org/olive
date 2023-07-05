use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .out_dir(out_dir.clone())
        .type_attribute("api.ProcessDefinitions", "#[derive(serde::Deserialize, serde::Serialize)]")
        .type_attribute("api.ProcessInstance", "#[derive(serde::Deserialize, serde::Serialize)]")
        .type_attribute("api.FlowNode", "#[derive(serde::Deserialize, serde::Serialize)]")
        .compile(&["../../proto/api/api.proto", "../../proto/server/olive.proto"], &["../../proto"])
        .unwrap_or_else(|e| panic!("Failed to compile protos {:?}", e));
}
