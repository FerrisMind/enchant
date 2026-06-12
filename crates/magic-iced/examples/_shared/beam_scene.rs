#![allow(dead_code)]

use iced::widget::Id;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeamSceneKind {
    Demo,
    Unidirectional,
    Bidirectional,
    MultipleInputs,
    MultipleOutputs,
}

pub const CONTAINER: &str = "beam-container";

pub fn measure_ids(kind: BeamSceneKind) -> Vec<Id> {
    match kind {
        BeamSceneKind::Demo => vec![
            Id::new(CONTAINER),
            Id::new("beam-n1"),
            Id::new("beam-n2"),
            Id::new("beam-n3"),
            Id::new("beam-n4"),
            Id::new("beam-n5"),
            Id::new("beam-n6"),
            Id::new("beam-n7"),
        ],
        BeamSceneKind::Unidirectional | BeamSceneKind::Bidirectional => {
            vec![Id::new(CONTAINER), Id::new("beam-left"), Id::new("beam-right")]
        }
        BeamSceneKind::MultipleInputs => vec![
            Id::new(CONTAINER),
            Id::new("beam-l1"),
            Id::new("beam-l2"),
            Id::new("beam-l3"),
            Id::new("beam-l4"),
            Id::new("beam-l5"),
            Id::new("beam-center"),
            Id::new("beam-user"),
        ],
        BeamSceneKind::MultipleOutputs => vec![
            Id::new(CONTAINER),
            Id::new("beam-user"),
            Id::new("beam-center"),
            Id::new("beam-r1"),
            Id::new("beam-r2"),
            Id::new("beam-r3"),
            Id::new("beam-r4"),
            Id::new("beam-r5"),
        ],
    }
}
