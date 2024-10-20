use {
    crate::{
        endpoint::{EndpointDirection, EndpointHandle, EndpointInfo},
        performer::{
            Endpoint, EndpointError, Performer, PerformerEndpoint, __seal_performer_endpoint,
        },
        value::ValueRef,
    },
    sealed::sealed,
};

/// An endpoint for input events.
#[derive(Debug, Copy, Clone)]
pub struct InputEvent {
    handle: EndpointHandle,
}

/// An endpoint for output events.
#[derive(Debug, Copy, Clone)]
pub struct OutputEvent {
    handle: EndpointHandle,
}

#[sealed]
impl PerformerEndpoint for InputEvent {
    fn make(
        handle: EndpointHandle,
        endpoint: EndpointInfo,
    ) -> Result<Endpoint<Self>, EndpointError> {
        if endpoint.direction() != EndpointDirection::Input {
            return Err(EndpointError::DirectionMismatch);
        }

        if endpoint.as_event().is_none() {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        Ok(Endpoint(InputEvent { handle }))
    }
}

#[sealed]
impl PerformerEndpoint for OutputEvent {
    fn make(handle: EndpointHandle, endpoint: EndpointInfo) -> Result<Endpoint<Self>, EndpointError>
    where
        Self: Sized,
    {
        if endpoint.direction() != crate::endpoint::EndpointDirection::Output {
            return Err(EndpointError::DirectionMismatch);
        }

        if endpoint.as_event().is_none() {
            return Err(EndpointError::EndpointTypeMismatch);
        }

        Ok(Endpoint(Self { handle }))
    }
}

pub fn post_event(
    performer: &mut Performer,
    Endpoint(InputEvent { handle }): Endpoint<InputEvent>,
    event: ValueRef<'_>,
) -> Result<(), EndpointError> {
    let type_index = performer
        .endpoints
        .get(&handle)
        .ok_or(EndpointError::EndpointDoesNotExist)?
        .as_event()
        .ok_or(EndpointError::EndpointTypeMismatch)?
        .type_index(event.ty())
        .ok_or(EndpointError::DataTypeMismatch)?;

    event.with_bytes(|bytes| {
        performer.ptr.add_input_event(handle, type_index, bytes);
    });

    Ok(())
}

pub fn fetch_events(
    performer: &mut Performer,
    Endpoint(OutputEvent { handle }): Endpoint<OutputEvent>,
    mut callback: impl FnMut(usize, ValueRef<'_>),
) -> Result<usize, EndpointError> {
    let types = performer
        .endpoints
        .get(&handle)
        .and_then(|endpoint| endpoint.as_event())
        .map(|endpoint| endpoint.types())
        .expect("endpoint should exist and be an event endpoint");

    let mut events = 0;

    performer
        .ptr
        .iterate_output_events(handle, |frame_offset, _, type_index, data| {
            let ty = types.get(usize::from(type_index));
            debug_assert!(ty.is_some(), "Invalid type index from Cmajor");

            if let Some(ty) = ty {
                callback(frame_offset, ValueRef::new_from_slice(ty.as_ref(), data));
                events += 1;
            }
        });

    Ok(events)
}
