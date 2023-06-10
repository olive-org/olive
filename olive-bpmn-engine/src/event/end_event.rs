//! # End Event flow node
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::bpmn::schema::{EndEvent as Element, FlowNodeType};
use crate::event::ProcessEvent;
use crate::flow_node::{self, Action, FlowNode, IncomingIndex};

/// End Event flow node
pub struct EndEvent {
    element: Arc<Element>,
    state: State,
    waker: Option<Waker>,
    event_broadcaster: Option<broadcast::Sender<ProcessEvent>>,
}

impl EndEvent {
    /// Creates new End Event flow node
    pub fn new(element: Element) -> Self {
        Self {
            element: Arc::new(element),
            state: State::Ready,
            waker: None,
            event_broadcaster: None,
        }
    }
}

/// Node state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum State {
    Ready,
    Complete,
    Done,
}

impl FlowNode for EndEvent {
    fn set_state(&mut self, state: flow_node::State) -> Result<(), flow_node::StateError> {
        match state {
            flow_node::State::EndEvent(state) => {
                self.state = state;
                Ok(())
            }
            _ => Err(flow_node::StateError::InvalidVariant),
        }
    }

    fn get_state(&mut self) -> flow_node::State {
        flow_node::State::EndEvent(self.state.clone())
    }

    fn element(&self) -> Box<dyn FlowNodeType> {
        Box::new(self.element.as_ref().clone())
    }

    fn incoming(&mut self, _index: IncomingIndex) {
        // Any incoming triggered is good enough to
        // complete EndEvent
        self.state = State::Complete;
        if let Some(waker) = self.waker.take() {
            waker.wake();
        }
    }

    fn set_process(&mut self, process: crate::process::Handle) {
        self.event_broadcaster.replace(process.event_broadcast());
    }
}

impl From<Element> for EndEvent {
    fn from(element: Element) -> Self {
        Self::new(element)
    }
}

impl Stream for EndEvent {
    type Item = Action;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.state {
            State::Ready => {
                self.waker.replace(cx.waker().clone());
                Poll::Pending
            }
            State::Complete => {
                self.state = State::Done;
                self.waker.replace(cx.waker().clone());
                if let Some(event_broadcaster) = self.event_broadcaster.as_ref() {
                    let _ = event_broadcaster.send(ProcessEvent::End);
                }
                Poll::Ready(Some(Action::Complete))
            }
            // There's no way to wake up EndEvent from this state
            // as end means end, period.
            State::Done => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::bpmn::parse;
    use crate::event::ProcessEvent;
    use crate::model;
    use crate::test::Mailbox;
    use olive_internal_macros as olive_im;

    #[olive_im::test]
    async fn end_throws_event() {
        let definitions = parse(include_str!("../../../definitions/start_flows.bpmn")).unwrap();
        let model = model::Model::new(definitions).spawn().await;

        let handle = model.processes().await.unwrap().pop().unwrap();
        let mut mailbox = Mailbox::new(handle.event_receiver());

        assert!(handle.start().await.is_ok());

        assert!(mailbox.receive(|e| matches!(e, ProcessEvent::End)).await);

        model.terminate().await;
    }
}
