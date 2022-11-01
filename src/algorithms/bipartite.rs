use std::{fmt, iter};
use std::collections::HashSet;
use std::fmt::Formatter;
use crate::algorithms::bipartite::NodeColor::*;
use crate::algorithms::bipartite::MatchingState::*;
use crate::types::{InitInfo, Message, PnAlgorithm, State};

/// Bipartite maximal matching algorithm
/// WARNING: Requires that the input network is bipartite wrt. even/odd nodes!
pub struct BipartiteMaximalMatching;

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

#[derive(Clone, PartialEq)]
enum MatchingState {
    Ur,
    Mr(u32),
    Us,
    Ms(u32),
}

impl fmt::Debug for MatchingState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Ur => write!(f, "UR"),
            Mr(i) => write!(f, "MR({})", i + 1),
            Us => write!(f, "US"),
            Ms(i) => write!(f, "MS({})", i + 1),
        }
    }
}

#[derive(Clone)]
pub struct BpState {
    degree: u32,
    color: NodeColor,
    round: u32,
    matching_state: MatchingState,
    m_set: HashSet<u32>,
    x_set: HashSet<u32>,
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.matching_state.fmt(f)
    }
}

impl PartialEq for BpState {
    fn eq(&self, other: &Self) -> bool {
        self.matching_state == other.matching_state
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum BpMessage {
    Noop,
    Proposal,
    Accept,
    Matched,
}

impl Message for BpMessage {}

impl PnAlgorithm<BpState, BpMessage> for BipartiteMaximalMatching {
    // `impl` convenience requires #![feature(type_alias_impl_trait)] and nightly Rust for now
    type MsgIter = Box<dyn Iterator<Item=BpMessage>>;

    fn name() -> String {
        "Bipartite Maximal Matching".into()
    }

    fn init(info: &InitInfo) -> BpState {
        let degree = info.node_degree;
        let color = info.node_id.into();

        let x_set = match color {
            White => HashSet::new(),
            Black => HashSet::from_iter(0..degree),
        };

        BpState {
            degree,
            color,
            round: 0,
            matching_state: Ur,
            m_set: HashSet::new(),
            x_set,
        }
    }

    fn send(state: &BpState) -> Self::MsgIter {
        match state {
            BpState { degree, color: White, round, matching_state: Ur, .. } if round % 2 == 0 && round < degree => {
                let r = *round; // Clone by dereference
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
        let mut result = state.clone();
        result.round += 1;

        let msg: Vec<_> = messages.collect();

        let index = msg
            .iter()
            .enumerate()
            .find(|(_, m)| *m == &BpMessage::Accept)
            .map(|e| e.0);

        match state {
            // White nodes
            BpState { degree, color: White, round, matching_state: Ur, .. } if round % 2 == 0 && round + 1 > *degree => {
                result.matching_state = Us;
            }
            BpState { color: White, round, matching_state: Mr(i), .. } if round % 2 == 0 => {
                result.matching_state = Ms(*i);
            }
            BpState { color: White, round, matching_state: Ur, .. } if round % 2 != 0 && index.is_some() => {
                result.matching_state = Mr(index.unwrap() as u32);
            }
            // Black nodes
            BpState { color: Black, round, matching_state: Ur, m_set, .. } if round % 2 != 0 && !m_set.is_empty() => {
                result.matching_state = Ms(*m_set.iter().min().unwrap());
            }
            BpState { color: Black, round, matching_state: Ur, x_set, .. } if round % 2 != 0 && x_set.is_empty() => {
                result.matching_state = Us;
            }
            BpState { color: Black, round, matching_state: Ur, .. } if round % 2 == 0 => {
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
            _ => ()
        };

        result
    }
}
