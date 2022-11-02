/*
 * (c) Dennis Marttinen 2022
 *
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

mod bipartite;
mod isomorphic;
mod mvc_3approx;

// Re-exports to allow direct access to the algorithms
pub use bipartite::BipartiteMaximalMatching;
pub use isomorphic::IsomorphicNeighborhood;
pub use mvc_3approx::Mvc3approx;
