use {
    crate::{
        endpoint::{EndpointDirection, EndpointTypeIndex},
        performer::{
            Endpoint, EndpointError, Performer, PerformerEndpoint, __seal_performer_endpoint,
        },
        value::{types::Type, Value},
    },
    real_time::fifo,
    sealed::sealed,
    std::marker::PhantomData,
};

/// An endpoint for input events.
pub struct InputEvent<T = Value> {
    types: Vec<Type>,
    tx: fifo::Producer<(EndpointTypeIndex, Value)>,
    _marker: PhantomData<T>,
}

impl Endpoint<InputEvent<Value>> {
    /// Post an event to the endpoint.
    pub fn post(&self, value: impl Into<Value>) -> Result<(), EndpointError> {
        let value = value.into();

        let index = self
            .inner
            .types
            .iter()
            .position(|t| t.as_ref() == value.ty())
            .map(EndpointTypeIndex::from)
            .ok_or(EndpointError::DataTypeMismatch)?;

        self.inner
            .tx
            .push((index, value))
            .map_err(|_e| EndpointError::FailedToSendMessageToPerformer)?;

        Ok(())
    }
}

#[sealed]
impl PerformerEndpoint for InputEvent<Value> {
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
            .as_event()
            .ok_or(EndpointError::EndpointTypeMismatch)?;

        let (tx, rx) = fifo::fifo(4);

        let input_event = InputEvent {
            types: endpoint.types().to_vec(),
            tx,
            _marker: PhantomData,
        };

        performer.inputs.push(Box::new(move |performer| {
            let mut n_to_pop_before_yielding = 4;
            while let Some(value) = rx.pop_ref() {
                if n_to_pop_before_yielding == 0 {
                    break;
                }
                n_to_pop_before_yielding -= 1;

                let (index, value) = &*value;
                value.with_bytes(|bytes| performer.add_input_event(handle, *index, bytes));
            }
        }));

        Ok(Endpoint { inner: input_event })
    }
}
