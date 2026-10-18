pub mod acquire;
pub mod app;
pub mod archive;
pub mod attachment_cache;
pub mod attachments;
pub mod catalog;
pub mod cli;
pub mod client;
pub mod collection;
pub mod compatibility;
pub mod config;
pub mod content;
pub mod credentials;
pub mod deadline;
#[cfg(windows)]
pub mod dispatch;
pub mod doctor;
pub mod domain;
pub mod error;
pub mod events;
pub mod export;
pub mod history;
pub mod installation;
pub mod instance_creation;
pub mod launch;
pub mod ledger;
pub mod media;
pub mod media_https;
pub mod media_source;
pub mod media_transport;
pub mod message;
#[cfg(windows)]
#[allow(unsafe_code)] // Audited Win32 object boundary; all other modules inherit deny.
pub mod native_job;
pub mod output;
#[cfg(windows)]
pub mod owned_http;
pub mod platform;
pub mod process;
pub mod qq;
pub mod read;
pub mod receipts;
pub mod references;
pub mod registry;
#[cfg(windows)]
pub mod retention;
#[cfg(windows)]
pub mod run_job;
pub mod runtime_config;
pub mod runtime_store;
pub mod schema;
pub mod session;
pub mod skill;
pub mod start_intent;
pub mod startup;
pub mod state;
pub mod stream_output;
pub mod transport;
pub mod validation;
pub mod watch;
pub mod websocket;
pub mod write;
