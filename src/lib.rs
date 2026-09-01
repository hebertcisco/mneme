//! mneme — crash-safe shared-memory engine over a Markdown vault.
//!
//! Markdown is the cortex (source of truth). `.mneme/` is the hippocampus
//! (SQLite FTS, ACT-R activation, journal). Vault language defaults to English;
//! source bodies in other languages are stored as-is with a `lang` tag.
//!
//! SPDX-License-Identifier: MIT OR Apache-2.0

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
pub mod lang;
pub mod note;
pub mod recall;
pub mod vault;

pub use error::MnemeError;
pub use vault::Vault;
