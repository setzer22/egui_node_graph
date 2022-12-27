#![forbid(unsafe_code)]

use slotmap::{SecondaryMap, SlotMap};

pub type SVec<T> = smallvec::SmallVec<[T; 4]>;

/// Contains the main definitions for the node graph model.
pub mod graph;
pub use graph::{Graph, GraphDropConnectionError, NodeDropConnectionError};

/// Type declarations for the different id types (node, input, output)
pub mod id_type;
pub use id_type::*;

/// Implements the index trait for the Graph type, allowing indexing by all
/// three id types
pub mod index_impls;

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

pub mod vertical_port;
pub use vertical_port::{VerticalPort, VerticalInputPort};

pub mod column_node;
pub use column_node::*;

/// Several traits that must be implemented by the user to customize the
/// behavior of this library.
pub mod traits;
pub use traits::*;

mod utils;

mod color_hex_utils;
