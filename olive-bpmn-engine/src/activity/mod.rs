//! Activity
use std::collections::HashMap;
use std::sync::Arc;
use std::task::{Poll, Waker};

use factory::ParameterizedFactory;
use futures::Stream;
use num_bigint::BigInt;
use serde::{Deserialize, Serialize};
use streamunordered::StreamUnordered;
use tokio::sync::{broadcast, oneshot, watch};

use crate::bpmn::schema::{
    self, ActivityType, BaseElementType, DataAssociationType, DataInput, DataOutput,
    DocumentElementContainer, Expr, FlowNodeType, FormalExpression, InputOutputSpecification,
    InputOutputSpecificationType, InputSetType, LoopCharacteristics,
    MultiInstanceLoopCharacteristics, MultiInstanceLoopCharacteristicsInputDataItem,
    MultiInstanceLoopCharacteristicsLoopCardinality,
    MultiInstanceLoopCharacteristicsOutputDataItem, OutputSetType, SequenceFlow,
    StandardLoopCharacteristics, StandardLoopCharacteristicsLoopCondition,
};
use crate::data_object::{self, DataObject};
use crate::flow_node::{self, Action, FlowNode, IncomingIndex, OutgoingIndex, StateError};
use crate::language::MultiLanguageEngine;
use crate::process::{self, Log};

pub trait Activity: FlowNode {
    /// Signals execution request
    fn execute(&mut self);

    /// Reports input sets
    ///
    /// Default implementation does nothing
    #[allow(unused_variables)]
    fn input_sets(&mut self, input_sets: Vec<InputSet>) {}

    /// Polls for output sets
    ///
    /// [`ActivityContainer`] expects this function to be non-idempotent, that is, the
    /// implementation should not return the same output set twice. The output sets stored
    /// in the implementation should be moved out via `Some(...)`.
    ///
    /// Default implementation returns `None`
    fn take_output_sets(&mut self) -> Option<Vec<OutputSet>> {
        None
    }
}

type NamedDataObjects = HashMap<String, Box<dyn DataObject>>;
type InputDataItem = Option<(Option<String>, Box<dyn DataObject>)>;

/// Optionally named input set
pub type InputSet = (Option<String>, Vec<Box<dyn DataObject>>);

/// Optionally named output set
pub type OutputSet = (Option<String>, Vec<Box<dyn DataObject>>);

pub struct ActivityContainer<T, E, F>
where
    T: Activity,
    E: ActivityType + Clone + Unpin,
    F: ParameterizedFactory<Item = T, Parameter = E> + Send + Clone + Unpin,
{
    state: InnerState,
    variant: Variant<T>,
    flow_nodes: StreamUnordered<T>,
    flow_node_tokens: Vec<usize>,
    element: E,
    engine: Arc<MultiLanguageEngine>,
    notifier: broadcast::Sender<Completion>,
    notifier_receiver: broadcast::Receiver<Completion>,
    log_broadcast: Option<broadcast::Sender<Log>>,
    waker: Option<Waker>,
    waker_sender: watch::Sender<Option<Waker>>,
    waker_receiver: watch::Receiver<Option<Waker>>,
    flow_node_maker: F,
    process: Option<process::Handle>,
    properties: HashMap<String, Box<dyn DataObject>>,
    // Per-instance input data items:
    // (token, input_data_item) => data_object
    input_data_items: HashMap<(usize, Option<String>), Box<dyn DataObject>>,
}

enum Variant<T> {
    // preparing instances
    Initializing {
        // returns activity with an optional (named) input_data_item
        activities: oneshot::Receiver<(IncomingIndex, Vec<(InputDataItem, T)>)>,
    },
    // waiting for data to be retrieved
    AwaitingData {
        incoming: IncomingIndex,
        data: oneshot::Receiver<NamedDataObjects>,
    },
    // waiting for an incoming flow
    Initialized,
    // incoming flow has been registered
    Ready {
        // for standard loop
        test_before: bool,
    },
    // activity is being executed
    Executing,
    // awaiting output completion
    AwaitingOutputs {
        // action to send once outputs are completed
        action: Option<Poll<Option<Action>>>,
        // this signals output propagation completion
        receiver: oneshot::Receiver<()>,
        // variant to change to
        next_variant: Box<Option<Variant<T>>>,
    },
    // loopCondition is being tested (standard loop)
    Testing,
    // error happened
    Errored,
    // done
    Done,
}

#[derive(Clone, Debug)]
enum Completion {
    Success(bool),
    Error,
}

impl<T, E, F> ActivityContainer<T, E, F>
where
    T: Activity + 'static,
    E: ActivityType + Clone + Unpin,
    F: ParameterizedFactory<Item = T, Parameter = E> + Send + Clone + Unpin + 'static,
{
    pub fn new(element: E, flow_node_maker: F) -> Self {
        let (notifier, notifier_receiver) = broadcast::channel(1);
        let (waker_sender, waker_receiver) = watch::channel(None);
        let mut activities = match element.loop_characteristics() {
            None => vec![flow_node_maker.create(element.clone())],
            Some(LoopCharacteristics::StandardLoopCharacteristics(_)) => {
                vec![flow_node_maker.create(element.clone())]
            }
            Some(LoopCharacteristics::MultiInstanceLoopCharacteristics(_)) => {
                // Can't prepare multi-instance activities at creation
                vec![]
            }
        };

        let mut flow_node_tokens = vec![];
        let mut flow_nodes = StreamUnordered::new();
        for activity in activities.drain(..) {
            flow_node_tokens.push(flow_nodes.insert(activity));
        }

        let properties = element
            .properties()
            .iter()
            .filter(|property| property.id().is_some())
            .map(|property| {
                (
                    property.id().as_ref().unwrap().clone(),
                    Box::new(data_object::Empty) as Box<dyn DataObject>,
                )
            })
            .collect();

        let input_data_items = HashMap::new();

        Self {
            state: InnerState {
                counter: BigInt::from(0usize),
            },
            variant: Variant::Initialized,
            flow_nodes,
            flow_node_tokens,
            element,
            engine: Arc::new(MultiLanguageEngine::new()),
            notifier,
            notifier_receiver,
            log_broadcast: None,
            waker: None,
            waker_sender,
            waker_receiver,
            flow_node_maker,
            process: None,
            properties,
            input_data_items,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State(Vec<flow_node::State>, InnerState);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InnerState {
    counter: BigInt,
}

impl<T, E, F> FlowNode for ActivityContainer<T, E, F>
where
    T: Activity + 'static,
    E: ActivityType + Clone + Unpin,
    F: ParameterizedFactory<Item = T, Parameter = E> + Send + Clone + Unpin + 'static,
{
    fn set_state(&mut self, state: flow_node::State) -> Result<(), StateError> {
        todo!()
    }

    fn get_state(&mut self) -> flow_node::State {
        todo!()
    }

    fn element(&self) -> Box<dyn FlowNodeType> {
        todo!()
    }
}

impl<T, E, F> Stream for ActivityContainer<T, E, F>
where
    T: Activity + 'static,
    E: ActivityType + Unpin + Clone,
    F: ParameterizedFactory<Item = T, Parameter = E> + Send + Clone + Unpin + 'static,
{
    type Item = Action;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        todo!()
    }
}
