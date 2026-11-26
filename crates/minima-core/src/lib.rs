//! The portable half of minima: providers over three wire formats, and the agent loop.
//!
//! A caller supplies the tools (`tools::Tools`), where events go (`Frontend`) and, optionally, a
//! token budget (`agent::Budget`). The `minima` CLI supplies `read`, `write`, `edit` and `bash`
//! under its sandbox; another host can supply its own set.

pub mod agent;
pub mod cache;
pub mod cancel;
pub mod config;
pub mod frontend;
pub mod fs;
pub mod price;
pub mod provider;
pub mod tools;
pub mod turn;

pub use agent::Agent;
pub use cancel::Cancel;
pub use config::Config;
pub use frontend::Frontend;
