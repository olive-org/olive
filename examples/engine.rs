#![warn(rust_2018_idioms)]

use olive_bpmn_engine::process::Log;
use tokio::{self, task};
use olive_bpmn_engine::bpmn::parse;
use olive_bpmn_engine::{model, context};
use serde_json::json;

#[tokio::main]
async fn main() {
    let definitions = parse(include_str!("../definitions/task_service.bpmn")).unwrap();
        let model = model::Model::new(definitions).spawn().await;

        let handle = model.processes().await.unwrap().pop().unwrap();
        let mut recevier = handle.log_receiver();

        assert!(handle.start().await.is_ok());

        let _ = task::spawn(async move {
            loop {
                match recevier.recv().await {
                    Ok(m) => {
                        println!("{:?}", m);
                        if let Log::FlowNodeExecuting { sender, .. } = m.clone() {
                            println!("reply service task");
                            let mut ctx = context::Context::new();
                            ctx.set_property("test", &json!("test".to_string()));
                            let _ = sender.send(Ok(Some(ctx)));
                        }
                        if let Log::FlowNodeCompleted { node, .. } = m.clone() {
                            if node.id().as_ref().unwrap().eq("end") {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }).await;

        model.terminate().await;
}