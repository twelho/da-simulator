/*
 * (c) Dennis Marttinen 2022
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

#![feature(type_alias_impl_trait)]

mod algorithms;
mod types;
mod simulator;

use types::*;
use std::time::{Duration};
use simulator::DaSimulator;

/// The main function. Take a look at the edge set format in the given examples to define your own
/// network, then select it together with the algorithm of your choice below. Run your simulation
/// with `cargo run --release`.
fn main() {
    // Edge sets for some generic networks
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

    let _triangle = [
        (0, 1), (1, 2), (0, 2)
    ];

    // Networks that are bipartite wrt. even/odd nodes
    let _bp_network1 = [
        (0, 1), (2, 1), (4, 1), (3, 2), (5, 2)
    ];

    let _bp_network2 = [
        (0, 1), (1, 2), (1, 4), (2, 3), (2, 5)
    ];

    let _square = [
        (0, 1), (1, 2), (2, 3), (0, 3)
    ];

    // A star network
    let _star_network: Vec<_> = (0..10).map(|i| (0, i + 1)).collect();

    // Select your algorithm here
    // type Algorithm = algorithms::IsomorphicNeighborhood<5>;
    // type Algorithm = algorithms::BipartiteMaximalMatching;
    type Algorithm = algorithms::Mvc3approx;

    // Select your network here
    let network = &_network2;

    let mut simulator: DaSimulator<Algorithm, _, _> =
        DaSimulator::from_network(network, Duration::from_secs(5));

    simulator.run(0);
    simulator.print();
}
