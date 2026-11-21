use {
    crate::{
        endpoint::{EndpointDirection, EndpointInfo},
        performer::{
            endpoints::{sealed::Sealed, EndpointKind},
            Endpoint, EndpointError, Performer,
        },
        value::StaticType,
    },
    std::marker::PhantomData,
};

/// An input stream.
pub struct InputStream<T>(PhantomData<T>)
where
    T: StreamType;

/// An output stream.
pub struct OutputStream<T>(PhantomData<T>)
where
    T: StreamType;

impl<T> Sealed for InputStream<T> where T: StreamType {}
impl<T> Sealed for OutputStream<T> where T: StreamType {}

impl<T> EndpointKind for InputStream<T>
where
    T: StreamType,
{
    type Binding = ();

    fn bind(info: &EndpointInfo) -> Result<(), EndpointError> {
        bind_stream_endpoint::<T>(info, EndpointDirection::Input)
    }
}

impl<T> EndpointKind for OutputStream<T>
where
    T: StreamType,
{
    type Binding = ();

    fn bind(info: &EndpointInfo) -> Result<(), EndpointError> {
        bind_stream_endpoint::<T>(info, EndpointDirection::Output)
    }
}

fn bind_stream_endpoint<T>(
    info: &EndpointInfo,
    expected_direction: EndpointDirection,
) -> Result<(), EndpointError>
where
    T: StreamType,
{
    if info.direction() != expected_direction {
        return Err(EndpointError::DirectionMismatch);
    }

    let stream = info
        .as_stream()
        .ok_or(EndpointError::EndpointTypeMismatch)?;

    if !stream.ty().is::<T>() {
        return Err(EndpointError::DataTypeMismatch);
    }

    Ok(())
}

impl<T> Endpoint<InputStream<T>>
where
    T: StreamType,
{
    pub(crate) fn write(&self, performer: &Performer, buffer: &[T]) {
        unsafe {
            performer
                .ptr
                .set_input_frames(self.handle_on(performer), buffer)
        }
    }
}

impl<T> Endpoint<OutputStream<T>>
where
    T: StreamType,
{
    pub(crate) fn read(&self, performer: &Performer, buffer: &mut [T]) {
        unsafe {
            performer
                .ptr
                .copy_output_frames(self.handle_on(performer), buffer)
        }
    }
}

/// A frame of a stream, either a scalar, or a vector of scalars.
pub trait StreamType: StaticType + Copy + sealed::Sealed {}

mod sealed {
    pub trait Sealed {}

    impl Sealed for i32 {}
    impl Sealed for i64 {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl<T, const N: usize> Sealed for [T; N] where T: Sealed {}
}

impl StreamType for i32 {}
impl StreamType for i64 {}
impl StreamType for f32 {}
impl StreamType for f64 {}
impl<T, const N: usize> StreamType for [T; N] where T: StreamType {}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            endpoint::StreamEndpoint,
            performer::endpoints::test,
            value::types::{Array, Type},
        },
    };

    fn input_stream_of(ty: impl Into<Type>) -> EndpointInfo {
        StreamEndpoint::new(
            test::id("s"),
            EndpointDirection::Input,
            ty.into(),
            Default::default(),
        )
        .into()
    }

    #[test]
    fn a_vector_stream_binds_to_an_array_of_matching_extent() {
        let info = input_stream_of(Array::new(Type::Float32, 2));

        assert!(InputStream::<[f32; 2]>::bind(&info).is_ok());
    }

    #[test]
    fn a_vector_stream_rejects_an_array_of_another_extent() {
        let info = input_stream_of(Array::new(Type::Float32, 2));

        assert!(matches!(
            InputStream::<[f32; 3]>::bind(&info),
            Err(EndpointError::DataTypeMismatch)
        ));
    }

    #[test]
    fn a_stream_rejects_an_endpoint_of_another_element_type() {
        let info = input_stream_of(Type::Int32);

        assert!(matches!(
            InputStream::<f32>::bind(&info),
            Err(EndpointError::DataTypeMismatch)
        ));
    }
}
