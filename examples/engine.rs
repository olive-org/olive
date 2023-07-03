#![warn(rust_2018_idioms)]

use bpmn_engine::bpmn::parse;
use bpmn_engine::process::Log;
use bpmn_engine::{context, model};
use serde_json::json;
use tokio::{self, task};

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let definitions = parse(include_str!("../definitions/task_service.bpmn")).unwrap();
    let model = model::Model::new(definitions).spawn().await;

    let handle = model.processes().await.unwrap().pop().unwrap();
    let mut recevier = handle.log_receiver();

    assert!(handle.start().await.is_ok());
    println!("1");

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
    })
    .await;

    model.terminate().await;
}
