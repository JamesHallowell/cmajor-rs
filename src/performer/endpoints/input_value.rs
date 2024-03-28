use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle},
        ffi::PerformerPtr,
        performer::{
            atomic::{AtomicF32, AtomicF64},
            endpoints::Endpoint,
            EndpointError, EndpointHandler, Performer, PerformerEndpoint,
            __seal_performer_endpoint,
        },
        value::{
            types::{Primitive, Type},
            Value,
        },
    },
    real_time::reader::realtime_reader,
    sealed::sealed,
    std::{
        any::TypeId,
        collections::HashMap,
        marker::PhantomData,
        sync::{
            atomic::{AtomicI32, AtomicI64, Ordering},
            Arc, Mutex,
        },
    },
};

/// An endpoint for input values.
pub struct InputValue<T = Value> {
    ty: Type,
    writer: Writer,
    _marker: PhantomData<fn(T)>,
}

type Writer = Arc<dyn Fn(Value) + Sync + Send>;

#[derive(Default)]
pub struct CachedInputValues {
    writers: HashMap<EndpointHandle, Writer>,
}

impl Endpoint<InputValue<bool>> {
    /// Set the value of the endpoint.
    pub fn set(&self, value: bool) {
        (self.inner.writer)(Value::Int32(if value { 1 } else { 0 }));
    }
}

impl Endpoint<InputValue<i32>> {
    /// Set the value of the endpoint.
    pub fn set(&self, value: i32) {
        (self.inner.writer)(value.into());
    }
}

impl Endpoint<InputValue<i64>> {
    /// Set the value of the endpoint.
    pub fn set(&self, value: i64) {
        (self.inner.writer)(value.into());
    }
}

impl Endpoint<InputValue<f32>> {
    /// Set the value of the endpoint.
    pub fn set(&self, value: f32) {
        (self.inner.writer)(value.into());
    }
}

impl Endpoint<InputValue<f64>> {
    /// Set the value of the endpoint.
    pub fn set(&self, value: f64) {
        (self.inner.writer)(value.into());
    }
}

impl Endpoint<InputValue<Value>> {
    /// Set the value of the endpoint. The type is checked at runtime.
    pub fn set(&self, value: impl Into<Value>) -> Result<(), EndpointError> {
        let value = value.into();
        if self.inner.ty.as_ref() != value.ty() {
            return Err(EndpointError::DataTypeMismatch);
        }
        (self.inner.writer)(value);
        Ok(())
    }
}

#[sealed]
impl<T> PerformerEndpoint for InputValue<T>
where
    T: 'static,
{
    fn make<Streams>(
        id: &str,
        performer: &mut Performer<Streams>,
    ) -> Result<Endpoint<Self>, EndpointError> {
        let (handle, endpoint) = performer
            .endpoints
            .get_by_id(id)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        if endpoint.direction() != EndpointDirection::Input {
            return Err(EndpointError::DirectionMismatch);
        }

        let endpoint = endpoint
            .as_value()
            .ok_or(EndpointError::EndpointTypeMismatch)?;

        let user_type = TypeId::of::<T>();
        if user_type != TypeId::of::<Value>() {
            if let Some(endpoint_type) = endpoint.ty().type_id() {
                if user_type != endpoint_type {
                    return Err(EndpointError::DataTypeMismatch);
                }
            } else {
                return Err(EndpointError::DataTypeMismatch);
            }
        }

        let writer = match performer.cached_input_values.writers.get(&handle) {
            Some(writer) => Arc::clone(writer),
            None => {
                let (writer, handler) = make_endpoint_connection(handle, endpoint.ty());
                performer
                    .cached_input_values
                    .writers
                    .insert(handle, Arc::clone(&writer));
                performer.inputs.push(handler);
                writer
            }
        };

        Ok(Endpoint {
            inner: InputValue {
                ty: endpoint.ty().clone(),
                writer,
                _marker: PhantomData,
            },
        })
    }
}

macro_rules! primitive_impl {
    ($handle:ident, $ty:ty, $atomic:ty) => {
        let reader = Arc::new(<$atomic>::default());
        let writer = Arc::clone(&reader);

        return (
            Arc::new(move |value| {
                if let Ok(value) = TryInto::<$ty>::try_into(value.as_ref()) {
                    writer.store(value, Ordering::Relaxed);
                }
            }),
            Box::new(move |performer: &mut PerformerPtr| {
                let value = reader.load(Ordering::Relaxed);
                unsafe { performer.set_input_value($handle, value.to_ne_bytes().as_ptr(), 0) };
            }),
        );
    };
}

fn make_endpoint_connection(handle: EndpointHandle, ty: &Type) -> (Writer, EndpointHandler) {
    match ty {
        Type::Primitive(Primitive::Bool) => {
            primitive_impl!(handle, i32, AtomicI32);
        }
        Type::Primitive(Primitive::Int32) => {
            primitive_impl!(handle, i32, AtomicI32);
        }
        Type::Primitive(Primitive::Int64) => {
            primitive_impl!(handle, i64, AtomicI64);
        }
        Type::Primitive(Primitive::Float32) => {
            primitive_impl!(handle, f32, AtomicF32);
        }
        Type::Primitive(Primitive::Float64) => {
            primitive_impl!(handle, f64, AtomicF64);
        }
        _ => {
            let (writer, reader) = realtime_reader(None);
            let writer = Mutex::new(writer);

            return (
                Arc::new(move |value| {
                    writer.lock().unwrap().set(Some(value));
                }),
                Box::new(move |performer| {
                    let reader = reader.lock();
                    if let Some(value) = reader.as_ref() {
                        value.with_bytes(|bytes| {
                            unsafe { performer.set_input_value(handle, bytes.as_ptr(), 0) };
                        });
                    }
                }),
            );
        }
    }
}
