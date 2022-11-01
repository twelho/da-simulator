use std::{fmt, iter};
use std::fmt::Formatter;
use crate::{InitInfo, Message, PnAlgorithm, State};

/// Isomorphic neighborhood gathering algorithm up to depth D
pub struct IsomorphicNeighborhood<const D: u32>;

#[derive(Clone, PartialEq)]
pub enum InState<const D: u32> {
    Count(u32, u32),
}

impl<const D: u32> State for InState<D> {
    fn is_output(&self) -> bool {
        match self {
            InState::Count(i, _) => *i == D,
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

#[derive(Clone, Debug)]
pub enum InMessage {
    Number(u32),
}

impl Message for InMessage {}

impl<const D: u32> PnAlgorithm<InState<D>, InMessage> for IsomorphicNeighborhood<D> {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = impl Iterator<Item=InMessage>;

    fn name() -> String {
        format!("Isomorphic Neighborhood (depth {D})")
    }

    fn init(info: &InitInfo) -> InState<D> {
        InState::Count(0, info.node_degree)
    }

    fn send(state: &InState<D>) -> Self::MsgIter {
        match *state {
            InState::Count(_, n) => Box::new(iter::repeat(InMessage::Number(n)))
        }
    }

    fn receive(state: &InState<D>, messages: impl Iterator<Item=InMessage>) -> InState<D> {
        match state {
            InState::Count(i, _) => {
                if *i == D {
                    state.clone()
                } else {
                    InState::Count(i + 1, messages.map(|m| match m {
                        InMessage::Number(n) => n
                    }).sum())
                }
            }
        }
    }
}
