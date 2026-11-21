//! Rust types whose Cmajor type is known at compile time.

use crate::value::types::{Array, Type};

/// A Rust type whose corresponding Cmajor type is known at compile time.
pub trait StaticType: Sized {
    /// The Cmajor type of the value.
    fn ty() -> Type;

    /// Writes the value into the start of `bytes`, returning a slice containing the written bytes.
    ///
    /// # Panics
    ///
    /// Panics if `bytes` is shorter than `Self::ty().size()`.
    fn write<'a>(&self, bytes: &'a mut [u8]) -> &'a [u8];

    /// Reads the value from the start of `bytes`, returning it with the bytes left over.
    ///
    /// # Panics
    ///
    /// Panics if `bytes` is shorter than `Self::ty().size()`.
    fn read(bytes: &[u8]) -> (Self, &[u8]);
}

impl StaticType for () {
    fn ty() -> Type {
        Type::Void
    }

    fn write<'a>(&self, _: &'a mut [u8]) -> &'a [u8] {
        &[]
    }

    fn read(bytes: &[u8]) -> (Self, &[u8]) {
        ((), bytes)
    }
}

macro_rules! scalar {
    ($rust:ty, $cmajor:expr) => {
        impl StaticType for $rust {
            fn ty() -> Type {
                $cmajor
            }

            fn write<'a>(&self, bytes: &'a mut [u8]) -> &'a [u8] {
                let native = self.to_ne_bytes();
                let written = &mut bytes[..native.len()];
                written.copy_from_slice(&native);
                written
            }

            fn read(bytes: &[u8]) -> (Self, &[u8]) {
                let (native, leftover) = bytes
                    .split_first_chunk()
                    .expect("bytes should hold the Cmajor type");
                (Self::from_ne_bytes(*native), leftover)
            }
        }
    };
}

scalar!(i32, Type::Int32);
scalar!(i64, Type::Int64);
scalar!(f32, Type::Float32);
scalar!(f64, Type::Float64);

impl StaticType for bool {
    fn ty() -> Type {
        Type::Bool
    }

    fn write<'a>(&self, bytes: &'a mut [u8]) -> &'a [u8] {
        i32::from(*self).write(bytes)
    }

    fn read(bytes: &[u8]) -> (Self, &[u8]) {
        let (value, leftover) = i32::read(bytes);
        (value != 0, leftover)
    }
}

impl<T, const N: usize> StaticType for [T; N]
where
    T: StaticType,
{
    fn ty() -> Type {
        Array::new(T::ty(), N).into()
    }

    fn write<'a>(&self, bytes: &'a mut [u8]) -> &'a [u8] {
        let mut len = 0;
        for elem in self {
            len += elem.write(&mut bytes[len..]).len();
        }
        &bytes[..len]
    }

    fn read(mut bytes: &[u8]) -> (Self, &[u8]) {
        let value = std::array::from_fn(|_| {
            let (elem, leftover) = T::read(bytes);
            bytes = leftover;
            elem
        });
        (value, bytes)
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
        let mut bytes = vec![0; T::ty().size()];
        value.write(&mut bytes);
        T::read(&bytes).0
    }

    fn decoded_dynamically<T: StaticType>(value: &T) -> Value {
        let ty = T::ty();
        let mut bytes = vec![0; ty.size()];
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

        assert_eq!(i32::read(&bytes).0, 1);
    }

    #[test]
    fn writing_into_a_larger_buffer_returns_only_the_written_bytes() {
        let mut bytes = [0xff; 16];

        let written = [[1_i32], [2]].write(&mut bytes);

        assert_eq!(<[[i32; 1]; 2]>::read(written), ([[1], [2]], &[][..]));
    }

    #[test]
    fn writing_into_a_larger_buffer_leaves_the_rest_untouched() {
        let mut bytes = [0xff; 16];

        Complex32 {
            real: 1.0,
            imag: 2.0,
        }
        .write(&mut bytes);

        assert_eq!(bytes[8..], [0xff; 8]);
    }

    #[test]
    fn reading_from_a_larger_buffer_reads_the_value_at_its_start() {
        let mut bytes = [0xff; 16];
        [[1_i32], [2]].write(&mut bytes);

        let (value, _) = <[[i32; 1]; 2]>::read(&bytes);

        assert_eq!(value, [[1], [2]]);
    }

    #[test]
    fn reading_returns_the_bytes_after_the_value() {
        let mut bytes = [0; 12];
        [1_i32, 2].write(&mut bytes);
        3_i32.write(&mut bytes[8..]);

        let (_, leftover) = <[i32; 2]>::read(&bytes);

        assert_eq!(i32::read(leftover).0, 3);
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
