//! # Service Task flow node
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::task::{Context, Poll, Waker};

use futures::stream::Stream;
use bpmn_schema::BaseElementType;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio::task;

use crate::activity::Activity;
use crate::bpmn::schema::{FlowNodeType, UserTask as Element};
use crate::context;
use crate::flow_node::{self, Action, FlowNode};
use crate::process::{self, ExecutionError, Log};

/// Script Task flow node
pub struct Task {
    context: context::Context,
    element: Arc<Element>,
    state: Arc<RwLock<State>>,
    waker: Option<Waker>,
    log_broadcast: Option<broadcast::Sender<Log>>,
    io_channel: broadcast::Sender<Result<Option<context::Context>, ExecutionError>>,
}

impl Task {
    /// Creates new User Task flow node
    pub fn new(element: Element) -> Self {
        let context = match element.extension_elements() {
            Some(el) => context::Context::from(el.clone()),
            None => context::Context::new(),
        };
        let (sender, _) = broadcast::channel(1);
        let state = Arc::new(RwLock::new(State::Initialized));
        Self {
            context,
            element: Arc::new(element),
            state,
            waker: None,
            log_broadcast: None,
            io_channel: sender,
        }
    }

    fn wake(&mut self) {
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    fn sync_set_state(&mut self, state: State) {
        if let Ok(mut state_) = self.state.write() {
            *state_ = state
        }
    }

    fn sync_get_state(&self) -> State {
        match self.state.read() {
            Ok(state) => state.clone(),
            Err(_) => State::Initialized,
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
    Completed,
    Errored,
    Done,
}

impl FlowNode for Task {
    fn set_state(&mut self, state: flow_node::State) -> Result<(), flow_node::StateError> {
        match state {
            flow_node::State::UserTask(state) => {
                self.sync_set_state(state);
                Ok(())
            }
            _ => Err(flow_node::StateError::InvalidVariant),
        }
    }

    fn get_state(&mut self) -> flow_node::State {
        flow_node::State::UserTask(self.sync_get_state())
    }

    fn element(&self) -> Box<dyn FlowNodeType> {
        Box::new(self.element.as_ref().clone())
    }

    fn set_process(&mut self, process: process::Handle) {
        let state = self.sync_get_state();
        if let State::Initialized = state {
            self.sync_set_state(State::Ready);
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
        self.sync_set_state(State::Execute);
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
        let state = self.sync_get_state();
        match state {
            State::Initialized => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
            State::Ready => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
            State::Execute => {
                self.sync_set_state(State::Executing);
                let waker = cx.waker().clone();
                let node = self.element();
                let log_broadcast = self.log_broadcast.clone();
                let context = Some(self.context.clone());
                let sender = self.io_channel.clone();
                task::spawn(async move {
                    if let Some(log_broadcast) = log_broadcast {
                        let _ = log_broadcast.send(Log::FlowNodeExecuting {
                            node,
                            context,
                            sender,
                        });
                    }
                    waker.wake();
                });
                Poll::Pending
            }
            State::Executing => {
                let waker = cx.waker().clone();

                let node = self.element();
                let mut context = self.context.clone();
                let new_state = Arc::clone(&self.state);
                let mut receiver = self.io_channel.subscribe();
                let log_broadcast = self.log_broadcast.clone();
                task::spawn(async move {
                    loop {
                        task::yield_now().await;
                        tokio::select! {
                            next = receiver.recv() => match next {
                                Ok(Ok(ctx)) => {
                                    if let Some(ref ctx) = ctx {
                                        context.merge(ctx);
                                    }
                                    *(new_state.write().unwrap()) = State::Completed;
                                    if let Some(log_broadcast) = log_broadcast {
                                        let _ = log_broadcast.send(Log::FlowNodeCompleted { node, context: Some(context) });
                                    }
                                    break;
                                }
                                Ok(Err(_)) => {
                                    *(new_state.write().unwrap()) = State::Errored;
                                    break;
                                },
                                Err(_) => {
                                    *(new_state.write().unwrap()) = State::Errored;
                                    break;
                                }
                            }
                        }
                    }
                    waker.wake();
                });
                Poll::Pending
            }
            State::Errored => {
                self.waker.replace(cx.waker().clone());
                Poll::Ready(None)
            }
            State::Completed => {
                self.sync_set_state(State::Ready);
                self.waker.replace(cx.waker().clone());
                Poll::Ready(Some(Action::Flow(
                    (0..self.element.outgoings().len()).collect(),
                )))
            }
            State::Done => Poll::Ready(None),
        }
    }
}

// #[cfg(test)]
// #[allow(unused_imports)]
// mod tests {
//     use crate::bpmn::parse;
//     use crate::bpmn::schema::*;
//     use crate::context::Context;
//     use crate::model;
//     use crate::process::Log;
//     use crate::test::Mailbox;
//     use crate::test::*;
//     use instant::Duration;
//     use internal_macros as olive_im;
//     use serde_json::json;
//     use tokio::sync::mpsc;
//     use tokio::task;
//     use tokio::time::sleep;

//     #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
//     async fn runs() {
//         let definitions = parse(include_str!("../../../../definitions/task_service.bpmn")).unwrap();
//         let model = model::Model::new(definitions).spawn().await;

//         let handle = model.processes().await.unwrap().pop().unwrap();
//         let mut recevier = handle.log_receiver();

//         assert!(handle.start().await.is_ok());

//         let _ = task::spawn(async move {
//             loop {
//                 match recevier.recv().await {
//                     Ok(m) => {
//                         if let Log::FlowNodeExecuting { sender, .. } = m.clone() {
//                             println!("reply service task");
//                             let _ = sender.send(Ok(None));
//                         }
//                         if let Log::FlowNodeCompleted { node, .. } = m.clone() {
//                             if node.id().as_ref().unwrap().eq("end") {
//                                 break;
//                             }
//                         }
//                     }
//                     Err(_) => break,
//                 }
//             }
//         }).await;

//         model.terminate().await;
//     }
// }
