// Public library surface — exposes core modules for integration tests and external use.
// The binary entry point (src/main.rs) remains a thin wrapper that imports from here.
pub mod config;
pub mod discord;
pub mod error;
pub mod params;
pub mod upgrade;
