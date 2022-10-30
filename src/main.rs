mod bipartite;

use std::{fmt, thread};
use petgraph::dot::{Config, Dot};
use petgraph::graph::EdgeReference;
use petgraph::prelude::*;
use crossbeam_channel::bounded;

pub struct Node {}

pub trait PnAlgorithm {
    fn init();
    fn send(n: &Node);
    fn receive(n: &Node);
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
    ]);

    graph
}

pub fn print_network<N: fmt::Debug>(graph: &Graph<N, (), Undirected>) {
    let pn = |er: EdgeReference<()>, source|
        graph.edges(if source { er.source() } else { er.target() })
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .enumerate()
            .find(|(i, e)| e == &er)
            .map(|(i, _)| i + 1)
            .expect("inconsistent edge");

    let edge_format = |_, er|
        format!("taillabel = \"{}\" headlabel = \"{}\" ", pn(er, true), pn(er, false));

    let dot = Dot::with_attr_getters(
        &graph,
        &[Config::EdgeNoLabel],
        &edge_format,
        &|_, _| String::new(),
    );

    println!("{:?}", dot);
}

// fn run_simulation<N: fmt::Debug + Send>(graph: &mut Graph<N, (), Undirected>) {
// }

fn main() {
    println!("Hello, world!");
    let mut graph = generate_graph();

    let things = graph.node_weights_mut();

    // graph.edges()
    //
    // for n in graph.raw_nodes() {
    //     n.
    // }

    // let (s, r) = bounded(1);

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
    print_network(&graph);
}
