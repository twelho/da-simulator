/*
 * (c) Dennis Marttinen 2022
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::{fmt, iter};
use std::collections::HashSet;
use super::bipartite::NodeColor::*;
use super::bipartite::MatchingState::*;
use crate::types::{Input, Message, DistributedAlgorithm, State};

/// Bipartite maximal matching algorithm in the PN model. **WARNING:** Requires that the input
/// network is bipartite wrt. even/odd nodes! Even nodes will be marked `White` and odd nodes
/// `Black` to establish the two partitions.
pub struct BipartiteMaximalMatching;

/// Helper enum for tracking node color
#[derive(Clone, Debug, PartialEq)]
enum NodeColor {
    White,
    Black,
}

impl From<u32> for NodeColor {
    fn from(i: u32) -> Self {
        // Even nodes are white and odd nodes are black
        match i % 2 == 0 {
            true => White,
            false => Black,
        }
    }
}

/// Enum for the four possible matching states
#[derive(Clone, PartialEq)]
enum MatchingState {
    // Unmatched and running
    Ur,
    // Matched over given port and running
    Mr(u32),
    // Unmatched and stopped
    Us,
    // Matched over given port and stopped
    Ms(u32),
}

impl fmt::Debug for MatchingState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ur => write!(f, "UR"),
            Mr(i) => write!(f, "MR({})", i + 1),
            Us => write!(f, "US"),
            Ms(i) => write!(f, "MS({})", i + 1),
        }
    }
}

/// Node state for the Bipartite Maximal Matching algorithm.
#[derive(Clone)]
pub struct BpState {
    degree: u32,
    color: NodeColor,
    round: u32,
    matching_state: MatchingState,
    m_set: HashSet<u32>,
    x_set: HashSet<u32>,
}

impl BpState {
    // Helper for the Minimum Vertex Cover 3-Approximation algorithm to determine final node state
    pub fn matched(&self) -> bool {
        match self.matching_state {
            Ms(_) => true,
            _ => false,
        }
    }
}

impl State for BpState {
    fn is_output(&self) -> bool {
        match self.matching_state {
            Us => true,
            Ms(_) => true,
            _ => false
        }
    }
}

impl fmt::Debug for BpState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.matching_state.fmt(f)
    }
}

impl PartialEq for BpState {
    fn eq(&self, other: &Self) -> bool {
        self.matching_state == other.matching_state
    }
}

/// Message format for the Bipartite Maximal Matching algorithm. Enables negotiation between nodes.
#[derive(Clone, Debug, PartialEq)]
pub enum BpMessage {
    Noop,
    Proposal,
    Accept,
    Matched,
}

impl Message for BpMessage {}

impl DistributedAlgorithm<BpState, BpMessage> for BipartiteMaximalMatching {
    // Boxing is required here since we return different implementors of this iterator
    type MsgIter = Box<dyn Iterator<Item=BpMessage>>;

    fn name() -> String {
        "Bipartite Maximal Matching".into()
    }

    fn init(info: &Input) -> BpState {
        let degree = info.node_degree;
        let color = info.node_id.into();

        // `x_set` is empty for white nodes, but populated with values for each port for black nodes
        let x_set = match color {
            White => HashSet::new(),
            Black => HashSet::from_iter(0..degree),
        };

        BpState {
            degree,
            color,
            round: 0,
            matching_state: Ur, // All nodes are initially unmatched and running
            m_set: HashSet::new(),
            x_set,
        }
    }

    fn send(state: &BpState) -> Self::MsgIter {
        // Match each of the states of the negotiation process separately. Look at the destructured
        // values to determine the conditions of taking the branch. By default, send `Noop` messages
        // to all neighbors.
        match state {
            BpState { degree, color: White, round, matching_state: Ur, .. } if round % 2 == 0 && round / 2 < *degree => {
                // In the original algorithm this is implemented using 2k, here we divide instead
                let r = round / 2;
                Box::new((0..).map(move |i| if i == r { BpMessage::Proposal } else { BpMessage::Noop }))
            }
            BpState { color: White, round, matching_state: Mr(_), .. } if round % 2 == 0 => {
                Box::new(iter::repeat(BpMessage::Matched))
            }
            BpState { color: Black, round, matching_state: Ur, m_set, .. } if round % 2 != 0 && !m_set.is_empty() => {
                let m = *m_set.iter().min().unwrap();
                Box::new((0..).map(move |i| if i == m { BpMessage::Accept } else { BpMessage::Noop }))
            }
            _ => Box::new(iter::repeat(BpMessage::Noop))
        }
    }

    fn receive(state: &BpState, messages: impl Iterator<Item=BpMessage>) -> BpState {
        // Initialize the resulting state as a copy of the current one
        let mut result = state.clone();
        result.round += 1; // Increment the round counter

        // Collect the messages from all ports into a vector
        let msg: Vec<_> = messages.collect();

        // Resolve the port number of a potential accept message
        let acc_index = msg
            .iter()
            .enumerate()
            .find(|(_, m)| *m == &BpMessage::Accept)
            .map(|e| e.0);

        // Match each of the states of the negotiation process separately. Look at the destructured
        // values to determine the conditions of taking the branch. By default, don't do any
        // additional modifications to the resulting state.
        match state {
            // White nodes
            BpState { degree, color: White, round, matching_state: Ur, .. } if round % 2 == 0 && round / 2 + 1 > *degree => {
                result.matching_state = Us;
            }
            BpState { color: White, round, matching_state: Mr(i), .. } if round % 2 == 0 => {
                result.matching_state = Ms(*i);
            }
            BpState { color: White, round, matching_state: Ur, .. } if round % 2 != 0 && acc_index.is_some() => {
                result.matching_state = Mr(acc_index.unwrap() as u32);
            }
            // Black nodes
            BpState { color: Black, round, matching_state: Ur, m_set, .. } if round % 2 != 0 && !m_set.is_empty() => {
                result.matching_state = Ms(*m_set.iter().min().unwrap());
            }
            BpState { color: Black, round, matching_state: Ur, x_set, .. } if round % 2 != 0 && x_set.is_empty() => {
                result.matching_state = Us;
            }
            BpState { color: Black, round, matching_state: Ur, .. } if round % 2 == 0 => {
                // Black nodes need to update both m_set and x_set on even rounds when unmatched
                msg
                    .iter()
                    .enumerate()
                    .for_each(|(i, e)| {
                        if e == &BpMessage::Matched {
                            result.x_set.remove(&(i as u32));
                        } else if e == &BpMessage::Proposal {
                            result.m_set.insert(i as u32);
                        }
                    });
            }
            _ => () // No special state changes by default
        };

        result
    }
}
