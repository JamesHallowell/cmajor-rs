use serde::Deserialize;

#[derive(Debug, Copy, Clone, Deserialize, PartialEq)]
pub enum Type {
    #[serde(rename = "void")]
    Void,

    #[serde(rename = "int32")]
    Int32,

    #[serde(rename = "int64")]
    Int64,

    #[serde(rename = "float32")]
    Float32,

    #[serde(rename = "float64")]
    Float64,

    #[serde(rename = "bool")]
    Bool,

    #[serde(rename = "string")]
    String,

    #[serde(rename = "vector")]
    Vector,

    #[serde(rename = "array")]
    Array,

    #[serde(rename = "object")]
    Object,
}

pub trait CmajorType: sealed::Sealed {
    const TYPE: Type;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R;

    fn from_bytes(bytes: &[u8]) -> Self;
}

impl CmajorType for () {
    const TYPE: Type = Type::Void;

    fn to_bytes<R>(&self, value: impl FnOnce(&[u8]) -> R) -> R {
        value(&[])
    }

    fn from_bytes(_: &[u8]) -> Self {
        ()
    }
}

impl CmajorType for i32 {
    const TYPE: Type = Type::Int32;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R {
        callback(&self.to_ne_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_ne_bytes(bytes[..4].try_into().expect("invalid bytes"))
    }
}

impl CmajorType for i64 {
    const TYPE: Type = Type::Int64;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R {
        callback(&self.to_ne_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_ne_bytes(bytes[..8].try_into().expect("invalid bytes"))
    }
}

impl CmajorType for f32 {
    const TYPE: Type = Type::Float32;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R {
        callback(&self.to_ne_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_ne_bytes(bytes[..4].try_into().expect("invalid bytes"))
    }
}

impl CmajorType for f64 {
    const TYPE: Type = Type::Float64;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R {
        callback(&self.to_ne_bytes())
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self::from_ne_bytes(bytes[..8].try_into().expect("invalid bytes"))
    }
}

impl CmajorType for bool {
    const TYPE: Type = Type::Bool;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R {
        callback(&[if *self { 1 } else { 0 }])
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bytes[0] != 0
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Complex<T = f32> {
    pub imag: T,
    pub real: T,
}

pub type Complex32 = Complex<f32>;
pub type Complex64 = Complex<f64>;

impl CmajorType for Complex32 {
    const TYPE: Type = Type::Object;

    fn to_bytes<R>(&self, callback: impl FnOnce(&[u8]) -> R) -> R {
        let slice = unsafe {
            std::slice::from_raw_parts(
                self as *const _ as *const u8,
                std::mem::size_of::<Complex32>(),
            )
        };
        callback(slice)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        assert!(bytes.len() >= std::mem::size_of::<Complex32>());

        let imag = f32::from_ne_bytes(bytes[0..4].try_into().expect("invalid bytes"));
        let real = f32::from_ne_bytes(bytes[4..8].try_into().expect("invalid bytes"));

        Self { imag, real }
    }
}

mod sealed {
    use super::*;

    pub trait Sealed {}

    impl Sealed for () {}
    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for bool {}
    impl Sealed for Complex32 {}
    impl Sealed for Complex64 {}
}
