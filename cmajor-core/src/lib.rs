#![warn(missing_docs)]

//! Rust bindings for the Cmajor JIT engine.

pub use {
    library::{Cmajor, LibraryError},
    program::{ast, ParseError, Program},
    serde_json as json,
};

pub mod diagnostic;
pub mod endpoint;
pub mod engine;
mod ffi;
mod library;
pub mod performer;
mod program;
pub mod value;

#[cfg(all(feature = "static", not(target_os = "macos")))]
compile_error!("The 'static' feature is only available on macOS currently.");
