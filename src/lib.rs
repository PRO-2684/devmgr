#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::nursery, clippy::pedantic, clippy::cargo)]

#[cfg(not(unix))]
compile_error!("devmgr only supports Unix-like platforms because it relies on termion.");

mod devcontainer;
mod exec;
mod terminal;

pub use devcontainer::Devcontainer;
