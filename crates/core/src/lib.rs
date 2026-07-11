pub mod adoption;
pub mod domain;
pub mod operation_graph;
pub mod reconciliation;
pub mod runtime;
pub mod storage;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
