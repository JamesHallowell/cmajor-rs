//! Rust types whose Cmajor type is known at compile time.

use crate::value::types::{Array, Type};

/// A Rust type whose corresponding Cmajor type is known at compile time.
pub trait StaticType: Sized {
    /// The number of bytes the value takes up in Cmajor's layout.
    const SIZE: usize;

    /// The Cmajor type of the value.
    fn ty() -> Type;

    /// Writes the value into bytes.
    fn write(&self, bytes: &mut [u8]);

    /// Reads the value from bytes.
    fn read(bytes: &[u8]) -> Self;
}

impl StaticType for () {
    const SIZE: usize = 0;

    fn ty() -> Type {
        Type::Void
    }

    fn write(&self, _: &mut [u8]) {}

    fn read(_: &[u8]) -> Self {}
}

macro_rules! scalar {
    ($rust:ty, $cmajor:expr) => {
        impl StaticType for $rust {
            const SIZE: usize = size_of::<$rust>();

            fn ty() -> Type {
                $cmajor
            }

            fn write(&self, bytes: &mut [u8]) {
                bytes[..Self::SIZE].copy_from_slice(&self.to_ne_bytes());
            }

            fn read(bytes: &[u8]) -> Self {
                let mut native = [0; size_of::<$rust>()];
                native.copy_from_slice(&bytes[..Self::SIZE]);
                Self::from_ne_bytes(native)
            }
        }
    };
}

scalar!(i32, Type::Int32);
scalar!(i64, Type::Int64);
scalar!(f32, Type::Float32);
scalar!(f64, Type::Float64);

impl StaticType for bool {
    const SIZE: usize = i32::SIZE;

    fn ty() -> Type {
        Type::Bool
    }

    fn write(&self, bytes: &mut [u8]) {
        i32::from(*self).write(bytes);
    }

    fn read(bytes: &[u8]) -> Self {
        i32::read(bytes) != 0
    }
}

impl<T, const N: usize> StaticType for [T; N]
where
    T: StaticType,
{
    const SIZE: usize = T::SIZE * N;

    fn ty() -> Type {
        Array::new(T::ty(), N).into()
    }

    fn write(&self, bytes: &mut [u8]) {
        for (index, elem) in self.iter().enumerate() {
            elem.write(&mut bytes[index * T::SIZE..]);
        }
    }

    fn read(bytes: &[u8]) -> Self {
        std::array::from_fn(|index| T::read(&bytes[index * T::SIZE..]))
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::value::{Complex32, Complex64, Value, ValueRef},
        std::fmt::Debug,
    };

    fn round_trip<T: StaticType>(value: &T) -> T {
        let mut bytes = vec![0; T::SIZE];
        value.write(&mut bytes);
        T::read(&bytes)
    }

    fn decoded_dynamically<T: StaticType>(value: &T) -> Value {
        let ty = T::ty();
        let mut bytes = vec![0; T::SIZE];
        value.write(&mut bytes);
        ValueRef::new_from_slice(ty.as_ref(), &bytes).to_owned()
    }

    fn assert_round_trips<T: StaticType + PartialEq + Debug>(value: T) {
        assert_eq!(round_trip(&value), value);
    }

    #[test]
    fn scalars_round_trip_through_their_encoding() {
        assert_round_trips(-7_i32);
        assert_round_trips(i64::MIN);
        assert_round_trips(0.25_f32);
        assert_round_trips(-1.5e300_f64);
        assert_round_trips(true);
        assert_round_trips(false);
        assert_round_trips(());
    }

    #[test]
    fn arrays_round_trip_through_their_encoding() {
        assert_round_trips([1_i32, 2, 3]);
        assert_round_trips([[1.0_f32, 2.0], [3.0, 4.0], [5.0, 6.0]]);
        assert_round_trips([true, false, true]);
    }

    #[test]
    fn complex_numbers_round_trip_through_their_encoding() {
        assert_round_trips(Complex32 {
            real: 1.0,
            imag: -2.0,
        });
        assert_round_trips(Complex64 {
            real: 0.5,
            imag: 4.0,
        });
    }

    #[test]
    fn a_bool_is_encoded_as_a_32_bit_integer() {
        let mut bytes = [0xff; 4];

        true.write(&mut bytes);

        assert_eq!(i32::read(&bytes), 1);
    }

    #[test]
    fn the_encoded_size_is_the_size_of_the_cmajor_type() {
        assert_eq!(<()>::SIZE, <()>::ty().size());
        assert_eq!(bool::SIZE, bool::ty().size());
        assert_eq!(i32::SIZE, i32::ty().size());
        assert_eq!(i64::SIZE, i64::ty().size());
        assert_eq!(f32::SIZE, f32::ty().size());
        assert_eq!(f64::SIZE, f64::ty().size());
        assert_eq!(<[[bool; 3]; 2]>::SIZE, <[[bool; 3]; 2]>::ty().size());
        assert_eq!(Complex32::SIZE, Complex32::ty().size());
        assert_eq!(Complex64::SIZE, Complex64::ty().size());
    }

    #[test]
    fn a_nested_array_is_a_cmajor_array_of_arrays() {
        assert_eq!(
            <[[i32; 3]; 2]>::ty(),
            Array::new(Array::new(Type::Int32, 3), 2).into()
        );
    }

    #[test]
    fn typed_and_dynamic_decoding_agree() {
        assert_eq!(decoded_dynamically(&true), true.into());
        assert_eq!(decoded_dynamically(&42_i64), 42_i64.into());
        assert_eq!(decoded_dynamically(&[1_i32, 2, 3]), [1_i32, 2, 3].into());

        let complex = Complex32 {
            real: 1.0,
            imag: 2.0,
        };
        assert_eq!(decoded_dynamically(&complex), complex.into());
    }
}
