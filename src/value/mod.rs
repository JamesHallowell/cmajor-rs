//! Support for Cmajor values.

pub mod types;
mod values;

pub use values::{
    ArrayValue, ArrayValueRef, Complex32, Complex64, ObjectValue, ObjectValueRef, Value, ValueRef,
};
