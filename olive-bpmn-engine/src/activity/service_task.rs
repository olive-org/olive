//! # Service Task flow node
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use futures::stream::Stream;
use olive_bpmn_schema::BaseElementType;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use tokio::task;

use crate::activity::Activity;
use crate::bpmn::schema::{FlowNodeType, ServiceTask as Element};
use crate::context;
use crate::flow_node::{self, Action, FlowNode};
use crate::process::{self, ExecutionError, Log};

/// Script Task flow node
pub struct Task {
    context: context::Context,
    element: Arc<Element>,
    state: State,
    waker: Option<Waker>,
    log_broadcast: Option<broadcast::Sender<Log>>,
    io_channel: (
        mpsc::Sender<Result<Option<context::Context>, ExecutionError>>,
        mpsc::Receiver<Result<Option<context::Context>, ExecutionError>>,
    ),
}

impl Task {
    /// Creates new Script Task flow node
    pub fn new(element: Element) -> Self {
        let (tx, rx) = mpsc::channel(10);
        let context = match element.extension_elements() {
            Some(el) => context::Context::from(el.clone()),
            None => context::Context::new(),
        };
        Self {
            context,
            element: Arc::new(element),
            state: State::Initialized,
            waker: None,
            log_broadcast: None,
            io_channel: (tx, rx),
        }
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }
}

/// Node state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum State {
    Initialized,
    Ready,
    Execute,
    Executing,
    Errored,
    Done,
}

impl FlowNode for Task {
    fn set_state(&mut self, state: flow_node::State) -> Result<(), flow_node::StateError> {
        match state {
            flow_node::State::ServiceTask(state) => {
                self.state = state;
                Ok(())
            }
            _ => Err(flow_node::StateError::InvalidVariant),
        }
    }

    fn get_state(&mut self) -> flow_node::State {
        flow_node::State::ServiceTask(self.state.clone())
    }

    fn element(&self) -> Box<dyn FlowNodeType> {
        Box::new(self.element.as_ref().clone())
    }

    fn set_process(&mut self, process: process::Handle) {
        if let State::Initialized = self.state {
            self.state = State::Ready;
            self.log_broadcast.replace(process.log_broadcast());
            self.wake();
        }
    }

    fn get_context(&self) -> Option<&context::Context> {
        Some(&self.context)
    }

    fn set_context(&mut self, ctx: &context::Context) {
        self.context.bond(ctx);
    }
}

impl Activity for Task {
    fn execute(&mut self) {
        self.state = State::Execute;
        self.wake();
    }
}

impl From<Element> for Task {
    fn from(element: Element) -> Self {
        Self::new(element)
    }
}

impl Stream for Task {
    type Item = Action;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.state {
            State::Initialized => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
            State::Ready => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
            State::Execute => {
                self.state = State::Executing;
                let wake = cx.waker().clone();
                wake.wake();
                Poll::Pending
            }
            State::Executing => {
                // let element = self.element.as_ref().clone();
                // self.waker.replace(cx.waker().clone());
                // let log_broadcast = self.log_broadcast.clone();
                // let sender = self.io_channel.0.clone();
                // let context = Some(self.context.clone());
                // if let Some(log_broadcast) = log_broadcast {
                //     let _ = log_broadcast.send(Log::FlowNodeExecuting {
                //         node: Box::new(element),
                //         context,
                //         sender,
                //     });
                // };

                // match self.io_channel.1.blocking_recv() {
                //     Some(Ok(ctx)) => {
                //         if let Some(ref ctx) = ctx {
                //             self.context.merge(ctx);
                //         }
                //         self.waker.replace(cx.waker().clone());
                //         self.state = State::Done;
                //         return Poll::Ready(Some(Action::Flow(
                //             (0..self.element.outgoings().len()).collect(),
                //         )));
                //     }
                //     Some(Err(_)) => {
                //         // match e {
                //         //     ExecutionError::Timeout => todo!(),
                //         //     ExecutionError::RestryErr { message, retries } => todo!(),
                //         // }
                //         self.state = State::Errored;
                //         return Poll::Ready(Some(Action::Complete));
                //     }
                //     None => Poll::Ready(None),
                // }
                Poll::Pending
            }
            State::Errored => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
            State::Done => {
                self.state = State::Ready;
                Poll::Ready(Some(Action::Complete))
            }
        }
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use crate::bpmn::parse;
    use crate::bpmn::schema::*;
    use crate::context::Context;
    use crate::model;
    use crate::process::Log;
    use crate::test::Mailbox;
    use crate::test::*;
    use instant::Duration;
    use olive_internal_macros as olive_im;
    use serde_json::json;
    use tokio::sync::mpsc;
    use tokio::task;
    use tokio::time::sleep;

    #[olive_im::test]
    async fn runs() {
        let definitions = parse(include_str!("test_models/task_service.bpmn")).unwrap();
        let model = model::Model::new(definitions).spawn().await;

        let handle = model.processes().await.unwrap().pop().unwrap();
        let mut recevier = handle.log_receiver();

        assert!(handle.start().await.is_ok());

        task::spawn(async move {
            loop {
                match recevier.recv().await {
                    Ok(m) => {
                        println!("==> {:?}\n", m);
                        if let Log::FlowNodeExecuting { sender, .. } = m {
                            let _ = sender.send(Ok(None));
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        let _ = sleep(Duration::from_secs(5)).await;

        model.terminate().await;
    }
}
