use std::cell::RefCell;
use std::fmt;
use crossbeam_channel::{bounded, Receiver, Sender};

pub trait Message: fmt::Debug + Send {}

pub trait State: Clone + fmt::Debug + PartialEq + Send {
    /// Determines if this state is a stopping state
    fn is_output(&self) -> bool;
}

#[derive(Debug)]
pub struct Edge<M: Message> {
    channel: RefCell<Option<(Sender<M>, Receiver<M>)>>,
    connected: RefCell<bool>,
}

impl<M: Message> Edge<M> {
    pub fn endpoint(&self) -> (Sender<M>, Receiver<M>) {
        if let Some((s, r)) = self.channel.take() {
            assert!(!self.connected.replace(true), "attempt to acquire third endpoint for edge");
            return (s, r);
        }

        let (s1, r1) = bounded(1);
        let (s2, r2) = bounded(1);
        self.channel.replace(Some((s1, r2)));
        (s2, r1)
    }
}

impl<M: Message> Default for Edge<M> {
    fn default() -> Self {
        Self {
            channel: RefCell::default(),
            connected: RefCell::default(),
        }
    }
}

impl<M: Message> PartialEq for Edge<M> {
    fn eq(&self, _: &Self) -> bool {
        true // All the edges are functionally identical and cannot be distinguished on their own
    }
}

#[allow(unused)]
pub struct InitInfo {
    pub node_count: u32,
    pub node_degree: u32,
}

/// Stateless Port Numbering model algorithm definition.
pub trait PnAlgorithm<S: State, M: Message> {
    /// Algorithm-provided iterator type for messages to send.
    type MsgIter: Iterator<Item=M>;

    /// Algorithm name retrieval function.
    fn name() -> String;

    /// Init function of the formal definition of a distributed algorithm.
    fn init(info: &InitInfo) -> S;

    /// Send function of the formal definition of a distributed algorithm.
    fn send(state: &S) -> Self::MsgIter;

    /// Receive function of the formal definition of a distributed algorithm.
    fn receive(state: &S, messages: impl Iterator<Item=M>) -> S;
}
