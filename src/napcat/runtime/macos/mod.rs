//! Apple Silicon runtime implementation, independent of the public command parser.
//! creation reserves an instance; layout/material prepare it; assembly/deploy
//! produce its dedicated application; run verifies the persisted service binding.
//! adapter/config contain the fixed Framework entry and configuration formats.
pub mod adapter;
pub mod assembly;
pub mod compatibility;
mod config;
pub mod creation;
pub mod deploy;
pub mod layout;
pub mod material;
pub mod restart;
pub mod run;
pub mod start;
pub mod stop;
