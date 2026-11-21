//! The pure-Rust Kanon domain: `Workspace` (SQLite + business rules),
//! floating local date/time types, and the clock providers.
//!
//! Nothing in here depends on an UI crate; the engine layer binds to it.

mod clock;
mod date;
mod document;
mod schema;
mod task;
mod workspace;

pub use clock::*;
pub use date::*;
pub use document::*;
pub use task::*;
pub use workspace::*;
