use std::iter;
use crate::{InitInfo, Message, PnAlgorithm, State};

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

    fn init(info: &InitInfo) -> BpState {
        BpState::None // TODO
    }

    fn send(state: &BpState) -> Self::MsgIter {
        Box::new(iter::repeat(BpMessage::Some)) // TODO
    }

    fn receive(state: &BpState, mut messages: impl Iterator<Item=BpMessage>) -> BpState {
        match messages.last() {
            None => BpState::None,
            Some(_) => BpState::Some,
        }
    }
}
