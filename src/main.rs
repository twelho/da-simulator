#![feature(type_alias_impl_trait)]

mod bipartite;

use std::{cmp, fmt, thread};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Barrier, Condvar};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::thread::Thread;
use std::time::{Duration, Instant};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DefaultIx, EdgeReference};
use petgraph::prelude::*;
use crossbeam_channel::{bounded, RecvError, RecvTimeoutError, SendError, SendTimeoutError};
use crossbeam_channel::{Sender, Receiver};
use petgraph::IntoWeightedEdge;
use petgraph::visit::IntoEdgeReferences;
use crate::bipartite::{BipartiteMaximalMatching, BpMessage, BpState};

trait Message: fmt::Debug + Send {}

trait State: Clone + fmt::Debug + PartialEq + Send {
    /// Determines if this state is a stopping state
    fn is_output(&self) -> bool;
}

#[derive(Debug)]
struct Edge<M: Message> {
    channel: RefCell<Option<(Sender<M>, Receiver<M>)>>,
    connected: RefCell<bool>,
}

impl<M: Message> Edge<M> {
    fn endpoint(&self) -> (Sender<M>, Receiver<M>) {
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

struct InitInfo {
    node_count: u32,
}

trait PnAlgorithm<S: State, M: Message> {
    type MsgIter: Iterator<Item=M>;

    fn init(info: &InitInfo) -> S;
    fn send(state: &S) -> Self::MsgIter;

    /// PN state machine receive function. Implementors MUST consume all messages,
    /// otherwise execution of the algorithm will deadlock (for obvious reasons).
    fn receive(state: &S, messages: impl Iterator<Item=M>) -> S;
}

struct PnSimulator<A: PnAlgorithm<S, M>, S: State, M: Message> {
    a: PhantomData<A>,
    graph: Graph<S, Edge<M>, Undirected>,
    timeout: Duration,
}

impl<A: PnAlgorithm<S, M>, S: State, M: Message> PnSimulator<A, S, M> {
    fn from_network(edges: &[(u32, u32)], timeout: Duration) -> Self {
        let node_count = 1 + edges
            .iter()
            .map(|(a, b)| a.max(b))
            .max()
            .expect("no edges given");

        // Create a new undirected graph
        let mut graph = Graph::new_undirected();

        // Initialize and add nodes
        let init_info = InitInfo { node_count };
        let node_indices: Vec<_> = (0..node_count).map(|i|
            graph.add_node(A::init(&init_info))
        ).collect();

        // Initialize and add edges
        for elt in edges {
            let (source, target, weight) = elt.into_weighted_edge();
            let (source, target) = (source.into(), target.into());
            graph.add_edge(source, target, weight);
        }

        // Ensure that the graph is simple
        node_indices.into_iter().for_each(|i| {
            let mut uniq = HashSet::new();
            assert!(
                graph
                    .edges(i)
                    .map(|e| e.target())
                    .all(|t| uniq.insert(t)),
                "graph must be simple"
            );
        });

        Self {
            a: PhantomData,
            graph,
            timeout,
        }
    }

    fn edges(&self, node: NodeIndex<DefaultIx>) -> Vec<EdgeReference<Edge<M>>> {
        // The edges are iterated in reverse order in this library so some fiddling is needed here
        let mut vd = VecDeque::new();
        self.graph.edges(node).for_each(|e| vd.push_front(e));
        vd.into()
    }

    fn run(&mut self) {
        // Acquire the communication channels between nodes from the edges
        let channels: Vec<(Vec<_>, Vec<_>)> = self.graph.node_indices()
            .map(|i| self.edges(i).iter().map(|e| e.weight().endpoint()).unzip())
            .collect();

        let node_count = self.graph.node_count();
        let stop_count = Arc::new(AtomicU32::new(0));

        thread::scope(|s| {
            self.graph
                .node_weights_mut()
                .zip(channels.into_iter())
                .enumerate()
                .for_each(|(_, (state, (senders, receivers)))| {
                    let stop_atomic = Arc::clone(&stop_count);
                    let deadline = Instant::now() + self.timeout;

                    s.spawn(move || {
                        let mut stopping_state: Option<S> = None;

                        loop {
                            let result = senders
                                .iter()
                                .zip(A::send(&state))
                                .map(|(s, m)| s.send_deadline(m, deadline))
                                .collect::<Result<(), _>>().err();

                            match result {
                                None => {}
                                Some(e) => {
                                    if let SendTimeoutError::Timeout(_) = e {
                                        println!("Deadlock!")
                                    }

                                    // Message channel was closed, execution finished
                                    break;
                                }
                            }

                            let messages = receivers
                                .iter()
                                .map(|r| r.recv_deadline(deadline))
                                .collect::<Result<Vec<_>, _>>();

                            match messages {
                                Ok(m) => *state = A::receive(&state, m.into_iter()),
                                Err(e) => {
                                    if let RecvTimeoutError::Timeout = e {
                                        println!("Deadlock!")
                                    }

                                    // Message channel was closed, execution finished
                                    break;
                                }
                            }

                            // Invalid stopping state transition detection
                            if let Some(s) = &stopping_state {
                                assert!(state == s, "detected post-stop state transition");
                            } else if state.is_output() {
                                stopping_state = Some(state.clone());
                                stop_atomic.fetch_add(1, Ordering::Relaxed);
                            }

                            if stop_atomic.load(Ordering::Relaxed) >= node_count as u32 {
                                println!("Finished!");
                                break;
                            }
                        }

                        // Close channels to notify of completion
                        senders.into_iter().for_each(|s| drop(s));
                        receivers.into_iter().for_each(|s| drop(s));

                        println!("Final state: {:?}", state);
                    });
                });
        });
    }

    fn print(&self) {
        let pn = |er: EdgeReference<Edge<M>>, source|
            self.edges(if source { er.source() } else { er.target() })
                .into_iter()
                .enumerate()
                .find(|(_, e)| e == &er)
                .map(|(i, _)| i + 1)
                .expect("inconsistent edge");

        let edge_format = |_, er|
            format!("taillabel = \"{}\" headlabel = \"{}\" ", pn(er, true), pn(er, false));

        let dot = Dot::with_attr_getters(
            &self.graph,
            &[Config::EdgeNoLabel],
            &edge_format,
            &|_, _| String::new(),
        );

        println!("{:?}", dot);
    }
}

fn main() {
    println!("Hello, world!");

    type Algorithm = BipartiteMaximalMatching;

    let mut simulator: PnSimulator<Algorithm, _, _> = PnSimulator::from_network(&[
        (0, 2), (0, 1), (0, 3),
        (1, 2), (1, 3), (2, 3),
    ], Duration::from_secs(5));

    simulator.run();
    simulator.print();
}
