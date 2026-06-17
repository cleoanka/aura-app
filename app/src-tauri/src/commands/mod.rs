pub mod agents;
pub mod index;

pub use agents::{agent_detect, agent_install};
pub use index::{get_graph, index_vault, search_fts};
