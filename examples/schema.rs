#![warn(rust_2018_idioms)]

use bpmn_schema::*;

fn main() {
    let mut d = Definitions {
        ..Default::default()
    };

    let mut process = Process {
        id: Some("process1".to_string()),
        flow_elements: vec![FlowElement::StartEvent(StartEvent {
            id: Some("start".to_string()),
            ..Default::default()
        })],
        ..Default::default()
    };

    let end_event = FlowElement::EndEvent(EndEvent {
        id: Some("end".to_string()),
        ..Default::default()
    });

    process.flow_elements_mut().push(end_event);

    process
        .establish_sequence_flow(
            "start",
            "end",
            "test",
            Some(FormalExpression {
                content: Some("condition".into()),
                ..Default::default()
            }),
        )
        .unwrap();

    let seq_flow = process
        .find_by_id("test")
        .unwrap()
        .downcast_ref::<SequenceFlow>()
        .unwrap();
    assert_eq!(seq_flow.id(), &Some("test".to_string()));
    assert_eq!(seq_flow.source_ref(), "start");
    assert_eq!(seq_flow.target_ref(), "end");

    d.set_id(Some("1".to_string()));
    d.root_elements_mut().push(RootElement::Process(process));

    let out = serde_json::to_string(&d).unwrap();
    println!("{:?}", out);
}
