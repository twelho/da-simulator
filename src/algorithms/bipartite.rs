use std::iter;
use crate::types::{InitInfo, Message, PnAlgorithm, State};

/// Bipartite maximal matching algorithm
/// TODO: WIP
pub struct BipartiteMaximalMatching;

#[derive(Clone, Debug, PartialEq)]
pub enum BpState {
    None,
    Some,
}

impl State for BpState {
    fn is_output(&self) -> bool {
        *self == Self::Some
    }
}

#[derive(Clone, Debug)]
pub enum BpMessage {
    None,
    Some,
}

impl Message for BpMessage {}

impl PnAlgorithm<BpState, BpMessage> for BipartiteMaximalMatching {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = impl Iterator<Item=BpMessage>;

    fn name() -> String {
        "Bipartite Maximal Matching".into()
    }

    fn init(_info: &InitInfo) -> BpState {
        BpState::None // TODO
    }

    fn send(_state: &BpState) -> Self::MsgIter {
        Box::new(iter::repeat(BpMessage::Some)) // TODO
    }

    fn receive(_state: &BpState, messages: impl Iterator<Item=BpMessage>) -> BpState {
        match messages.last() {
            None => BpState::None,
            Some(_) => BpState::Some,
        }
    }
}
