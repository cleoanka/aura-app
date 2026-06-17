pub mod agents;
pub mod ai;
pub mod index;
pub mod modes;
pub mod settings;
pub mod vault;

pub use agents::{agent_detect, agent_install};
pub use ai::{ask, ask_consensus, cancel_job};
pub use index::{get_graph, index_vault, search_fts};
pub use modes::run_mode;
pub use settings::{get_settings, set_settings};
pub use vault::{list_notes, pick_vault_folder, read_note, search_hybrid, write_note};
