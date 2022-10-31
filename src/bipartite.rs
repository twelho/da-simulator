use petgraph::{Graph, Undirected};
use crate::{InitInfo, Message, PnAlgorithm, State};

pub struct BipartiteMaximalMatching;

#[derive(Debug)]
pub enum BipartiteState {
    None, // TODO
}

impl State for BipartiteState {}

#[derive(Debug)]
pub enum BipartiteMessage {
    None, // TODO
}

impl Message for BipartiteMessage {}

impl PnAlgorithm<BipartiteState, BipartiteMessage> for BipartiteMaximalMatching {
    fn init(info: &InitInfo) -> BipartiteState {
        BipartiteState::None // TODO
    }

    fn send<const D: usize>(state: &BipartiteState) -> [BipartiteMessage; D] {
        todo!()
    }

    fn receive<const D: usize>(state: BipartiteState, messages: [BipartiteMessage; D]) -> BipartiteState {
        todo!()
    }
}
