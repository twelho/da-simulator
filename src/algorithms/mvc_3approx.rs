use std::iter;
use crate::types::{InitInfo, Message, PnAlgorithm, State};

/// Minimum vertex cover 3-approximation algorithm
struct Mvc3approx {}

#[derive(Clone, Debug, PartialEq)]
struct Mvc3approxState {}

impl State for Mvc3approxState {
    fn is_output(&self) -> bool {
        todo!()
    }
}

#[derive(Clone, Debug)]
struct Mvc3approxMessage {}

impl Message for Mvc3approxMessage {}

impl PnAlgorithm<Mvc3approxState, Mvc3approxMessage> for Mvc3approx {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = impl Iterator<Item=Mvc3approxMessage>;

    fn name() -> String {
        "Minimum Vertex Cover 3-Approximation".into()
    }

    fn init(info: &InitInfo) -> Mvc3approxState {
        todo!()
    }

    fn send(state: &Mvc3approxState) -> Self::MsgIter {
        iter::repeat(Mvc3approxMessage{})
    }

    fn receive(state: &Mvc3approxState, messages: impl Iterator<Item=Mvc3approxMessage>) -> Mvc3approxState {
        todo!()
    }
}
