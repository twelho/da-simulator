mod bipartite;

use std::{cmp, fmt, thread};
use std::borrow::BorrowMut;
use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use petgraph::dot::{Config, Dot};
use petgraph::graph::{DefaultIx, EdgeReference};
use petgraph::prelude::*;
use crossbeam_channel::bounded;
use crossbeam_channel::{Sender, Receiver};
use petgraph::IntoWeightedEdge;
use petgraph::visit::IntoEdgeReferences;
use crate::bipartite::{BipartiteMaximalMatching, BipartiteMessage, BipartiteState};

trait Message: fmt::Debug {}

trait State: fmt::Debug + Send {}

#[derive(Debug)]
struct Edge<M: Message> {
    channel: Option<(Sender<M>, Receiver<M>)>,
    connected: bool,
}

impl<M: Message> Edge<M> {
    fn endpoint(&mut self) -> (Sender<M>, Receiver<M>) {
        assert!(!self.connected, "attempt to acquire third endpoint for edge");

        if let Some((s, r)) = self.channel.take() {
            self.connected = true;
            return (s, r);
        }

        let (s1, r1) = bounded(1);
        let (s2, r2) = bounded(1);
        self.channel = Some((s1, r2));
        (s2, r1)
    }
}

impl<M: Message> Default for Edge<M> {
    fn default() -> Self {
        Self {
            connected: bool::default(),
            channel: Option::default(),
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
    fn init(info: &InitInfo) -> S;
    fn send<const D: usize>(state: &S) -> [M; D];
    fn receive<const D: usize>(state: S, messages: [M; D]) -> S;
}

struct PnSimulator<A: PnAlgorithm<S, M>, S: State, M: Message> {
    a: PhantomData<A>,
    graph: Graph<S, Edge<M>, Undirected>,
}

impl<A: PnAlgorithm<S, M>, S: State, M: Message> PnSimulator<A, S, M> {
    fn from_network(edges: &[(u32, u32)]) -> Self {
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
        }
    }

    fn edges(&self, node: NodeIndex<DefaultIx>) -> Vec<EdgeReference<Edge<M>>> {
        // The edges are iterated in reverse order in this library so some fiddling is needed here
        let mut vd = VecDeque::new();
        self.graph.edges(node).for_each(|e| vd.push_front(e));
        vd.into()
    }

    fn run(&mut self) {
        // TODO: Edge "weights" (endpoint() calls) need to happen before giving the graph to node_weights_mut()

        // TODO: This won't work since e.endpoint() needs to be invoked twice per edge, the interior
        //  mutability pattern would work much better here, and then we don't need to deal with
        //  edge_weights_mut etc.
        let endpoints = self.graph.edge_weights_mut().map(|e| Some(e.endpoint())).collect::<Vec<_>>();

        // let r = (0..self.graph.node_count()).map(|index| {
        //     let edges = self.edges((index as u32).into());
        //
        //     self.graph.edge_references()
        //
        //     edges.clone()
        //         .iter()
        //         .map(|a| a.id())
        //         .map(|a| {
        //             let b = self.graph.edge_weight_mut(a).expect("non-existent edge").endpoint();
        //             b
        //         }).collect::<Vec<_>>()
        // });

        let states = self.graph.node_indices().zip(self.graph.node_weights_mut());


        thread::scope(|s| {
            states.for_each(|(index, state)| {
                let edges: Vec<_> = self.edges(index).into_iter().map(|e| e.id()).collect();

                let (senders, receivers) = edges.iter().map(|i| {
                    endpoints[i.index()]
                        .take().expect("???")
                }).unzip();

                let senders: Vec<_> = edges.iter().map(|i| senders[i.index()]).collect();
                let receivers: Vec<_> = edges.iter().map(|i| receivers[i.index()]).collect();

                let channels = edges
                    .into_iter()
                    .map(|a| self.graph.edge_weight_mut(a).expect("non-existent edge").endpoint());

                // senders.for_each(|s| );

                s.spawn(move || {
                    // *s += 1;
                    println!("Thing: {:?}", state);
                });
            });
        });
    }

    fn print(&self) {
        let pn = |er: EdgeReference<Edge<M>>, source|
            self.graph.edges(if source { er.source() } else { er.target() })
                .collect::<Vec<_>>() // TODO: Get rid of this collect
                .into_iter()
                .rev()
                .enumerate()
                .find(|(i, e)| e == &er)
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

fn generate_graph() -> Graph<i32, (), Undirected> {
    let mut graph = Graph::new_undirected();
    graph.add_node(3);
    graph.add_node(5);
    graph.add_node(9);
    graph.add_node(10);
    graph.extend_with_edges(&[
        (0, 2), (0, 1), (0, 3),
        (1, 2), (1, 3), (2, 3),
    ].map(|(a, b)| (a, b, ())));

    graph
}

fn main() {
    println!("Hello, world!");

    type Algorithm = BipartiteMaximalMatching;

    let mut simulator: PnSimulator<Algorithm, _, _> = PnSimulator::from_network(&[
        (0, 2), (0, 1), (0, 3),
        (1, 2), (1, 3), (2, 3),
    ]);

    simulator.run();

    simulator.print();

    let mut graph = generate_graph();

    let things = graph.node_weights_mut();

    thread::scope(|s| {
        things.for_each(|i| {
            s.spawn(move || {
                *i += 1;
                println!("Thing: {:?}", i);
            });
        });
    });

    // handles.for_each(|h| {h.join();});

    // for i in 0..10 {
    //     thread::spawn(move || {
    //         println!("{}", i)
    //     });
    // }

    // run_simulation(&mut graph);
    // print_network(&graph);
}
