//! Support for Cmajor values.

mod static_type;
pub mod types;
mod values;

pub use {
    static_type::StaticType,
    values::{
        ArrayValue, ArrayValueRef, Complex32, Complex64, ObjectValue, ObjectValueRef, StringHandle,
        Value, ValueRef,
    },
};
