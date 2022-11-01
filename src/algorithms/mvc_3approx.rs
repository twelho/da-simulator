use std::fmt;
use std::fmt::Formatter;
use crate::algorithms::{BipartiteMaximalMatching, BpMessage, BpState};
use crate::types::{InitInfo, Message, PnAlgorithm, State};

/// Minimum vertex cover 3-approximation algorithm
pub struct Mvc3approx {}

#[derive(Clone, PartialEq)]
pub struct Mvc3approxState {
    s1: BpState,
    s2: BpState,
}

impl State for Mvc3approxState {
    fn is_output(&self) -> bool {
        self.s1.is_output() && self.s2.is_output()
    }
}

impl fmt::Debug for Mvc3approxState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", (self.s1.matched() || self.s2.matched()) as u8)
    }
}

#[derive(Clone, Debug)]
pub struct Mvc3approxMessage {
    m1: BpMessage,
    m2: BpMessage,
}

impl Message for Mvc3approxMessage {}

impl PnAlgorithm<Mvc3approxState, Mvc3approxMessage> for Mvc3approx {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = impl Iterator<Item=Mvc3approxMessage>;

    fn name() -> String {
        "Minimum Vertex Cover 3-Approximation".into()
    }

    fn init(info: &InitInfo) -> Mvc3approxState {
        Mvc3approxState {
            s1: BipartiteMaximalMatching::init(&InitInfo {
                node_id: 0, // "Even" nodes
                ..*info
            }),
            s2: BipartiteMaximalMatching::init(&InitInfo {
                node_id: 1, // "Odd" nodes
                ..*info
            }),
        }
    }

    fn send(state: &Mvc3approxState) -> Self::MsgIter {
        BipartiteMaximalMatching::send(&state.s1).zip(BipartiteMaximalMatching::send(&state.s2))
            .map(|(m2, m1)| Mvc3approxMessage { m1, m2 })
    }

    fn receive(state: &Mvc3approxState, messages: impl Iterator<Item=Mvc3approxMessage>) -> Mvc3approxState {
        let (m1, m2): (Vec<_>, Vec<_>) = messages.map(|m| (m.m1, m.m2)).unzip();

        Mvc3approxState {
            s1: BipartiteMaximalMatching::receive(&state.s1, m1.into_iter()),
            s2: BipartiteMaximalMatching::receive(&state.s2, m2.into_iter()),
        }
    }
}
