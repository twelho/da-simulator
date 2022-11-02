/*
 * (c) Dennis Marttinen 2022
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use crossbeam_channel::{RecvTimeoutError, SendTimeoutError};
use petgraph::{Graph, IntoWeightedEdge, Undirected};
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DefaultIx, EdgeReference};
use petgraph::prelude::*;
use crate::types::*;

/// A highly parallel simulator capable of running arbitrary distributed algorithms of various
/// models of computation (PN, LOCAL, CONGEST) on networks constructed from arbitrary graphs.
pub struct DaSimulator<A: DistributedAlgorithm<S, M>, S: State, M: Message> {
    a: PhantomData<A>,
    // This is required to keep the algorithm in scope since it is stateless
    graph: Graph<S, Edge<M>, Undirected>,
    timeout: Duration,
}

impl<A: DistributedAlgorithm<S, M>, S: State, M: Message> DaSimulator<A, S, M> {
    /// Construct a new simulator that builds a new network from the given set of edges (the order
    /// of which determines the port numbering) and has the given timeout for deadlock prevention
    pub fn from_network(edges: &[(u32, u32)], timeout: Duration) -> Self {
        // Count the required amount of nodes from the given edges
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
        let node_indices: Vec<_> = node_degrees
            .into_iter()
            .enumerate()
            .map(|(node_id, node_degree)|
                graph.add_node(A::init(&Input {
                    node_id: node_id as u32,
                    node_count,
                    node_degree,
                }))
            ).collect();

        // Initialize and add edges
        for elt in edges {
            let (mut source, mut target, weight) = elt.into_weighted_edge();
            if source > target {
                // As a result of how `petgraph` operates under the hood, the port numbers
                // (edge ordering) gets messed up if these are the wrong way around
                std::mem::swap(&mut source, &mut target);
            }

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

    /// Retrieve the list of edges attached to the given node in order of port numbers
    fn edges(&self, node: NodeIndex<DefaultIx>) -> Vec<EdgeReference<Edge<M>>> {
        // The edges are iterated in reverse order in `petgraph` so some fiddling is needed here
        let mut vd = VecDeque::new();
        self.graph.edges(node).for_each(|e| vd.push_front(e));
        vd.into()
    }

    /// Run the simulation, optionally terminating after `round_limit` communication rounds if
    /// `round_limit > 0`. If `round_limit == 0`, run until natural termination.
    pub fn run(&mut self, round_limit: u32) {
        println!("\nSimulating the {} algorithm in a PN network with {} nodes and {} edges...",
                 A::name(), self.graph.node_count(), self.graph.edge_count());

        // Acquire the communication channels between the nodes from the edges
        let channels: Vec<(Vec<_>, Vec<_>)> = self.graph.node_indices()
            .map(|i| self.edges(i).iter().map(|e| e.weight().endpoint()).unzip())
            .collect();

        // Initialize some references for the threads
        let node_count = self.graph.node_count();
        let stop_count = Arc::new(AtomicU32::new(0));

        // A thread scope allows for spawning a set of threads and waiting for them to finish
        thread::scope(|s| {
            // Compose the necessary data for a single node thread. The "weight" of a node is the
            // payload it carries, in our case that is an instance of the state as defined by the
            // algorithm to run.
            self.graph
                .node_weights_mut()
                .zip(channels.into_iter())
                .enumerate()
                .for_each(|(i, (state, (senders, receivers)))| {
                    let stop_atomic = Arc::clone(&stop_count);
                    let deadline = Instant::now() + self.timeout;

                    // Spawn the node thread
                    s.spawn(move || {
                        // Track the stopping state for detecting invalid transitions after stopping
                        let mut stopping_state: Option<S> = None;
                        let mut iterations = 0;

                        loop {
                            // Send messages based on the current state to all neighbors
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

                                    // Message channel was closed, execution is finished
                                    break;
                                }
                            }

                            // Receive messages from all neighbors
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

                                    // Message channel was closed, execution is finished
                                    break;
                                }
                            }

                            if let Some(s) = &stopping_state {
                                // Invalid stopping state transition detection
                                assert!(state == s, "detected post-stop state transition");
                            } else if state.is_output() {
                                stopping_state = Some(state.clone());

                                // If the node reached a stopping state add
                                // it to the atomic counter of stopped nodes
                                stop_atomic.fetch_add(1, Ordering::Relaxed);
                            }

                            // If all nodes have reached a stopping state, stop the simulation
                            if stop_atomic.load(Ordering::Relaxed) >= node_count as u32 {
                                break;
                            }

                            // (Optional) communication round limiting
                            iterations += 1;
                            if round_limit > 0 && iterations >= round_limit {
                                break;
                            }
                        }

                        // Close channels to notify neighbor nodes of completion
                        senders.into_iter().for_each(|s| drop(s));
                        receivers.into_iter().for_each(|s| drop(s));
                    });
                });
        });

        let unfinished = self.graph.node_weights().filter(|s| !s.is_output()).count();
        if unfinished > 0 {
            eprintln!(
                "\nSimulation FAILED! Timeout reached with {} node(s) still running, states in the\n\
                resulting network are NOT final! Hint: check for deadlocks or increase the timeout.",
                unfinished
            )
        } else {
            println!("\nSimulation successful! All nodes reached stopping states.");
        }
    }

    /// Output the network in the [Graphviz DOT format](https://graphviz.org/doc/info/lang.html)
    pub fn print(&self) {
        // Function for resolving the port number of an edge
        let pn = |er: EdgeReference<Edge<M>>, source|
            self.edges(if source { er.source() } else { er.target() })
                .into_iter()
                .enumerate()
                .find(|(_, e)| e == &er)
                .map(|(i, _)| i + 1)
                .expect("inconsistent edge");

        // Helper for formatting an edge with port numbers
        let edge_format = |_, er|
            format!("taillabel = \"{}\" headlabel = \"{}\" ", pn(er, true), pn(er, false));

        // Serialize the internal graph to DOT format
        let dot = Dot::with_attr_getters(
            &self.graph,
            &[Config::EdgeNoLabel],
            &edge_format,
            &|_, _| String::new(),
        );

        // Print the serialization
        println!("\n{:?}", dot);
    }
}
