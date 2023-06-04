use std::sync::Arc;

use tokio::sync::{mpsc, broadcast};

use crate::bpmn::schema::{FlowNodeType, Process as Element};
use crate::event::ProcessEvent as Event;
use crate::model;

/// Process container
pub struct Process {
    element: Arc<Element>,
    model: model::Handle,
}

/// Control handle for a running process
#[derive(Clone)]
pub struct Handle {
    model: model::Handle,
    element: Arc<Element>,
    sender: mpsc::Sender<Request>,
    log_broadcast: broadcast::Sender<Log>,
    event_broadcast: broadcast::Sender<Event>,
}

pub enum Log {}
