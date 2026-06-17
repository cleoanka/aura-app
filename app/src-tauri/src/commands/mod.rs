pub mod agents;
pub mod index;
pub mod settings;
pub mod vault;

pub use agents::{agent_detect, agent_install};
pub use index::{get_graph, index_vault, search_fts};
pub use settings::{get_settings, set_settings};
pub use vault::{list_notes, pick_vault_folder, search_hybrid};
