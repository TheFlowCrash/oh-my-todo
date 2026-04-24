//! Application and library surface for `oh-my-todo`.
//!
//! The published crate provides the shared application core used by the `todo`
//! binary, including storage, domain modeling, CLI parsing, and Ratatui TUI
//! modules.

pub mod application;
pub mod cli;
pub mod domain;
pub mod storage;
pub mod tui;

pub use application::bootstrap::{AppContext, BootstrapOptions, bootstrap};
