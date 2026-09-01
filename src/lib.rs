//! mneme — crash-safe shared-memory engine over a Markdown vault.
//!
//! Markdown is the cortex (source of truth). `.mneme/` is the hippocampus
//! (SQLite FTS, ACT-R activation, journal). All durable notes are English.

pub mod atomic;
pub mod cli;
pub mod compact;
pub mod config;
pub mod consolidate;
pub mod doctor;
pub mod encode;
pub mod error;
pub mod graph;
pub mod index;
pub mod journal;
pub mod note;
pub mod recall;
pub mod vault;

pub use error::MnemeError;
pub use vault::Vault;
