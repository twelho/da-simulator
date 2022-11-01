use std::cell::RefCell;
use std::fmt;
use crossbeam_channel::{bounded, Receiver, Sender};

/// A `Message` is an object that can be sent over a single edge in the DA state machine
pub trait Message: fmt::Debug + Send {}

/// A `State` represents a configuration a single node can transition to in the DA state machine
pub trait State: Clone + fmt::Debug + PartialEq + Send {
    /// Determines if the state is a stopping state
    fn is_output(&self) -> bool;
}

/// An `Edge` describes a bidirectional communication channel between two nodes
#[derive(Debug)]
pub struct Edge<M: Message> {
    channel: RefCell<Option<(Sender<M>, Receiver<M>)>>,
    connected: RefCell<bool>,
}

impl<M: Message> Edge<M> {
    /// Acquire one endpoint of the edge, the returned `Sender` and `Receiver` pair can be used to
    /// communicate with the other end
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

// Manual implementation needed to avoid `Default` dependency on `M`
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

/// Underlying graph/node data to be passed to the `init` function. This is multi-purpose, and as
/// such algorithms operating in the PN model should disregard fields such as `node_id` as a source
/// of unique identifiers.
#[allow(unused)]
pub struct Input {
    pub node_id: u32,
    pub node_count: u32,
    pub node_degree: u32,
}

/// Programmatic representation of the formal definition of a distributed algorithm
pub trait DistributedAlgorithm<S: State, M: Message> {
    /// Algorithm-provided iterator type for a stream of messages to send
    type MsgIter: Iterator<Item=M>;

    /// Function to retrieve the name of the algorithm
    fn name() -> String;

    /// `init` function of the formal definition of a distributed algorithm. Takes in an input with
    /// graph/node details (but may choose to ignore it), and returns the initial state of a node.
    fn init(info: &Input) -> S;

    /// `send` function of the formal definition of a distributed algorithm. Takes in an immutable
    /// reference to the current state, and must produce an iterator of messages to be sent to each
    /// port of the node in order.
    fn send(state: &S) -> Self::MsgIter;

    /// `receive` function of the formal definition of a distributed algorithm. Takes in an
    /// immutable reference to the current state as well as an iterator with the messages received
    /// from each port in order, and must produce a new state that the node then transitions to.
    fn receive(state: &S, messages: impl Iterator<Item=M>) -> S;
}
