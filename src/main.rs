#![feature(type_alias_impl_trait)]

mod bipartite;
mod isomorphic;

use std::{fmt, thread};

use std::cell::RefCell;
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

use std::sync::{Arc};
use std::sync::atomic::{AtomicU32, Ordering};

use std::time::{Duration, Instant};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DefaultIx, EdgeReference};
use petgraph::prelude::*;
use crossbeam_channel::{bounded, RecvTimeoutError, SendTimeoutError};
use crossbeam_channel::{Sender, Receiver};
use petgraph::IntoWeightedEdge;

use crate::bipartite::{BipartiteMaximalMatching};
use crate::isomorphic::{IsomorphicNeighborhood};

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

#[allow(unused)]
struct InitInfo {
    node_count: u32,
    node_degree: u32,
}

/// Stateless Port Numbering model algorithm definition.
trait PnAlgorithm<S: State, M: Message> {
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

        // Figure out the node degrees in advance for the initialization
        let mut node_degrees = vec![0; node_count as usize];
        edges.iter().flat_map(|(a, b)| [a, b]).for_each(|i| node_degrees[*i as usize] += 1);

        // Create a new undirected graph
        let mut graph = Graph::new_undirected();

        // Initialize and add nodes
        let node_indices: Vec<_> = (0..node_count).map(|i|
            graph.add_node(A::init(&InitInfo {
                node_count,
                node_degree: node_degrees[i as usize],
            }))
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
        println!("\nSimulating the {} algorithm in a PN network with {} nodes and {} edges...",
                 A::name(), self.graph.node_count(), self.graph.edge_count());

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
                .for_each(|(i, (state, (senders, receivers)))| {
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
                                        eprintln!("Thread {i}: send timeout!")
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
                                        println!("Thread {i}: receive timeout!")
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
                                break;
                            }
                        }

                        // Close channels to notify of completion
                        senders.into_iter().for_each(|s| drop(s));
                        receivers.into_iter().for_each(|s| drop(s));
                    });
                });
        });

        let unfinished = self.graph.node_weights().filter(|s| !s.is_output()).count();
        if unfinished > 0 {
            eprintln!(
                "\nSimulation FAILED! Timeout reached with {} nodes still running, node states in the\n\
                resulting graph are NOT final! Hint: check for deadlocks or increase the timeout.",
                unfinished
            )
        } else {
            println!("\nSimulation successful! All nodes reached stopping states.");
        }
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

        println!("\n{:?}", dot);
    }
}

fn main() {
    // 0, 1, 2,  3,  4,   5
    // 2, 6, 17, 56, 163, 521
    // 2, 6, 17, 52, 148, 445
    // type Algorithm = IsomorphicNeighborhood<0>;

    type Algorithm = BipartiteMaximalMatching;

    let _network1 = [
        (0, 2), (0, 1), (0, 3),
        (1, 2), (1, 3), (2, 3),
    ];

    let _network2 = [
        (0, 1), (0, 2), (1, 3), (2, 3), (2, 4), (3, 4),
        (1, 5), (4, 5), (4, 6), (5, 6), (6, 7), (6, 8)
    ];

    let _network3 = [
        (0, 1), (0, 2), (1, 3), (2, 3), (2, 4), (3, 4),
        (1, 5), (4, 5), (4, 6), (5, 7), (6, 7)
    ];

    let mut simulator: PnSimulator<Algorithm, _, _> =
        PnSimulator::from_network(&_network2, Duration::from_secs(5));

    simulator.run();
    simulator.print();
}
