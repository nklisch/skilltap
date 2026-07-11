pub mod adoption;
pub mod domain;
pub mod executor;
pub mod instructions;
pub mod marketplace;
pub mod native_config;
pub mod operation_graph;
pub mod reconciliation;
pub mod runtime;
pub mod skill;
pub mod skill_compatibility;
pub mod skill_lifecycle;
pub mod skill_source;
pub mod storage;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
