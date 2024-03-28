use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle},
        performer::{EndpointError, Performer},
        value::types::{Primitive, Type},
    },
    sealed::sealed,
    std::marker::PhantomData,
};

impl<Input> Performer<(Input, ())> {
    /// Bind an output stream to the performer.
    pub fn with_output_stream<T>(
        self,
        id: impl AsRef<str>,
    ) -> Result<Performer<(Input, OutputStream<T>)>, EndpointError>
    where
        T: StreamType,
    {
        let (handle, endpoint) = self
            .endpoints
            .get_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.direction() != EndpointDirection::Output {
            return Err(EndpointError::DirectionMismatch);
        }

        let stream_endpoint = endpoint
            .as_stream()
            .ok_or(EndpointError::EndpointTypeMismatch)?;

        match stream_endpoint.ty() {
            Type::Primitive(primitive) => {
                if T::EXTENT != 1 || &T::ELEMENT != primitive {
                    return Err(EndpointError::DataTypeMismatch);
                }
            }
            Type::Array(array) => {
                if T::EXTENT != array.len() || &Type::Primitive(T::ELEMENT) != array.elem_ty() {
                    return Err(EndpointError::DataTypeMismatch);
                }
            }
            _ => return Err(EndpointError::EndpointTypeMismatch),
        }

        let Self {
            inner,
            endpoints,
            inputs,
            outputs,
            cached_input_values,
            streams: (input, ()),
        } = self;

        Ok(Performer {
            inner,
            endpoints,
            inputs,
            outputs,
            cached_input_values,
            streams: (
                input,
                OutputStream {
                    handle,
                    _marker: PhantomData,
                },
            ),
        })
    }
}

impl<Output> Performer<((), Output)> {
    /// Bind an input stream to the performer.
    pub fn with_input_stream<T>(
        self,
        id: impl AsRef<str>,
    ) -> Result<Performer<(InputStream<T>, Output)>, EndpointError>
    where
        T: StreamType,
    {
        let (handle, endpoint) = self
            .endpoints
            .get_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.direction() != EndpointDirection::Input {
            return Err(EndpointError::DirectionMismatch);
        }

        let stream_endpoint = endpoint
            .as_stream()
            .ok_or(EndpointError::EndpointTypeMismatch)?;

        match stream_endpoint.ty() {
            Type::Primitive(primitive) => {
                if T::EXTENT != 1 || &T::ELEMENT != primitive {
                    return Err(EndpointError::DataTypeMismatch);
                }
            }
            Type::Array(array) => {
                if T::EXTENT != array.len() || &Type::Primitive(T::ELEMENT) != array.elem_ty() {
                    return Err(EndpointError::DataTypeMismatch);
                }
            }
            _ => return Err(EndpointError::EndpointTypeMismatch),
        }

        let Self {
            inner,
            endpoints,
            inputs,
            outputs,
            cached_input_values,
            streams: ((), output),
        } = self;

        Ok(Performer {
            inner,
            endpoints,
            inputs,
            outputs,
            cached_input_values,
            streams: (
                InputStream {
                    handle,
                    _marker: PhantomData,
                },
                output,
            ),
        })
    }
}

/// An input stream.
pub struct InputStream<T> {
    handle: EndpointHandle,
    _marker: PhantomData<T>,
}

/// An output stream.
pub struct OutputStream<T> {
    handle: EndpointHandle,
    _marker: PhantomData<T>,
}

impl<T, Output> Performer<(InputStream<T>, Output)>
where
    T: StreamType,
{
    /// Write to the performers input stream.
    pub fn write_stream(&mut self, frames: &[T]) {
        unsafe { self.write_stream_unchecked(self.streams.0.handle, frames) }
    }
}

impl<T, Input> Performer<(Input, OutputStream<T>)>
where
    T: StreamType,
{
    /// Read from the performers output stream.
    pub fn read_stream(&mut self, frames: &mut [T]) {
        unsafe { self.read_stream_unchecked(self.streams.1.handle, frames) }
    }
}

#[sealed]
pub trait StreamType: Copy {
    const ELEMENT: Primitive;
    const EXTENT: usize;
}

#[sealed]
impl StreamType for i32 {
    const ELEMENT: Primitive = Primitive::Int32;
    const EXTENT: usize = 1;
}

#[sealed]
impl StreamType for i64 {
    const ELEMENT: Primitive = Primitive::Int64;
    const EXTENT: usize = 1;
}

#[sealed]
impl StreamType for f32 {
    const ELEMENT: Primitive = Primitive::Float32;
    const EXTENT: usize = 1;
}

#[sealed]
impl StreamType for f64 {
    const ELEMENT: Primitive = Primitive::Float64;
    const EXTENT: usize = 1;
}

#[sealed]
impl<T, const EXTENT: usize> StreamType for [T; EXTENT]
where
    T: StreamType,
{
    const ELEMENT: Primitive = T::ELEMENT;
    const EXTENT: usize = EXTENT;
}
