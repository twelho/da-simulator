use std::fmt;
use std::fmt::Formatter;
use super::bipartite::{BipartiteMaximalMatching, BpMessage, BpState};
use crate::types::{Input, Message, DistributedAlgorithm, State};

/// Minimum vertex cover 3-approximation algorithm in the PN model. Leverages the Bipartite Maximal
/// Matching algorithm in a virtual bipartite network configuration.
pub struct Mvc3approx {}

/// Node state for the MVC 3-approx. algorithm. Tracks the states of both virtual nodes.
#[derive(Clone, PartialEq)]
pub struct Mvc3approxState {
    s1: BpState,
    s2: BpState,
}

impl State for Mvc3approxState {
    fn is_output(&self) -> bool {
        self.s1.is_output() && self.s2.is_output() // Require both instances to be stopped
    }
}

impl fmt::Debug for Mvc3approxState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Output 1 if either of the two virtual nodes ends up matched
        write!(f, "{}", (self.s1.matched() || self.s2.matched()) as u8)
    }
}

/// Message format for the MVC 3-approx. algorithm. Tracks the messages for both virtual edges.
#[derive(Clone, Debug)]
pub struct Mvc3approxMessage {
    m1: BpMessage,
    m2: BpMessage,
}

impl Message for Mvc3approxMessage {}

impl DistributedAlgorithm<Mvc3approxState, Mvc3approxMessage> for Mvc3approx {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = impl Iterator<Item=Mvc3approxMessage>;

    fn name() -> String {
        "Minimum Vertex Cover 3-Approximation".into()
    }

    fn init(info: &Input) -> Mvc3approxState {
        Mvc3approxState {
            s1: BipartiteMaximalMatching::init(&Input {
                node_id: 0, // "Even" nodes
                ..*info
            }),
            s2: BipartiteMaximalMatching::init(&Input {
                node_id: 1, // "Odd" nodes
                ..*info
            }),
        }
    }

    fn send(state: &Mvc3approxState) -> Self::MsgIter {
        // Swap the messages during sending to make the virtual network bipartite
        BipartiteMaximalMatching::send(&state.s1).zip(BipartiteMaximalMatching::send(&state.s2))
            .map(|(m2, m1)| Mvc3approxMessage { m1, m2 })
    }

    fn receive(state: &Mvc3approxState, messages: impl Iterator<Item=Mvc3approxMessage>) -> Mvc3approxState {
        // Receive the joined messages for both virtual network partitions and unzip them
        let (m1, m2): (Vec<_>, Vec<_>) = messages.map(|m| (m.m1, m.m2)).unzip();

        // Generate a new state by running the receive function for both virtual nodes individually
        Mvc3approxState {
            s1: BipartiteMaximalMatching::receive(&state.s1, m1.into_iter()),
            s2: BipartiteMaximalMatching::receive(&state.s2, m2.into_iter()),
        }
    }
}
