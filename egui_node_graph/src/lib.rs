#![forbid(unsafe_code)]

use slotmap::{SecondaryMap, SlotMap};

pub type SVec<T> = smallvec::SmallVec<[T; 4]>;

/// Contains the main definitions for the node graph model.
pub mod graph;
pub use graph::*;

/// Type declarations for the different id types (node, input, output)
pub mod id_type;
pub use id_type::*;

/// Implements the index trait for the Graph type, allowing indexing by all
/// three id types
pub mod index_impls;

/// Implementing the main methods for the `Graph`
pub mod graph_impls;

/// Custom error types, crate-wide
pub mod error;
pub use error::*;

/// The main struct in the library, contains all the necessary state to draw the
/// UI graph
pub mod ui_state;
pub use ui_state::*;

/// The node finder is a tiny widget allowing to create new node types
pub mod node_finder;
pub use node_finder::*;

/// The inner details of the egui implementation. Most egui code lives here.
pub mod editor_ui;
pub use editor_ui::*;

/// Several traits that must be implemented by the user to customize the
/// behavior of this library.
pub mod traits;
pub use traits::*;

mod utils;

mod color_hex_utils;
mod scale;
