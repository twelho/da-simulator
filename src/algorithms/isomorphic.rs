use std::{fmt, iter};
use std::fmt::Formatter;
use crate::{Input, Message, DistributedAlgorithm, State};

/// Isomorphic neighborhood gathering algorithm up to depth D in the PN model. This algorithm it is
/// just a functional test and does nothing useful, but it can be used to answer a certain quiz :)
pub struct IsomorphicNeighborhood<const D: u32>;

/// Node state for the Isomorphic Neighborhood algorithm. Variant format: `Count(<rounds>, <sum>)`
#[derive(Clone, PartialEq)]
pub enum InState<const D: u32> {
    Count(u32, u32),
}

impl<const D: u32> State for InState<D> {
    fn is_output(&self) -> bool {
        match self {
            InState::Count(i, _) => *i == D, // Target depth reached
        }
    }
}

impl<const D: u32> fmt::Debug for InState<D> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            InState::Count(_, n) => write!(f, "{n}")
        }
    }
}

/// Message format for the Isomorphic Neighborhood algorithm. Used for sending individual integers.
#[derive(Clone, Debug)]
pub enum InMessage {
    Number(u32),
}

impl Message for InMessage {}

impl<const D: u32> DistributedAlgorithm<InState<D>, InMessage> for IsomorphicNeighborhood<D> {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = impl Iterator<Item=InMessage>;

    fn name() -> String {
        format!("Isomorphic Neighborhood (depth {D})")
    }

    fn init(info: &Input) -> InState<D> {
        InState::Count(0, info.node_degree) // Initialize sum to node degree
    }

    fn send(state: &InState<D>) -> Self::MsgIter {
        match *state {
            // All neighbors get the same number
            InState::Count(_, n) => Box::new(iter::repeat(InMessage::Number(n)))
        }
    }

    fn receive(state: &InState<D>, messages: impl Iterator<Item=InMessage>) -> InState<D> {
        match state {
            InState::Count(i, _) => {
                if state.is_output() {
                    state.clone() // Don't transition to another state if we've stopped
                } else {
                    // Next round with a sum of the numbers from all neighbors
                    InState::Count(i + 1, messages.map(|m| match m {
                        InMessage::Number(n) => n
                    }).sum())
                }
            }
        }
    }
}
