#![warn(missing_docs)]

//! Rust bindings for the Cmajor JIT engine.

pub use {
    library::{Cmajor, LibraryError},
    program::{ParseError, Program},
};

pub mod diagnostic;
pub mod endpoint;
pub mod engine;
mod ffi;
mod library;
pub mod performer;
mod program;
pub mod value;
