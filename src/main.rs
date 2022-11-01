#![feature(type_alias_impl_trait)]

mod algorithms;
mod types;
mod simulator;

use types::*;
use std::time::{Duration};
use simulator::PnSimulator;

fn main() {
    // 0, 1, 2,  3,  4,   5
    // 2, 6, 17, 56, 163, 521
    // 2, 6, 17, 52, 148, 445
    // type Algorithm = algorithms::IsomorphicNeighborhood<0>;

    type Algorithm = algorithms::BipartiteMaximalMatching;

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
