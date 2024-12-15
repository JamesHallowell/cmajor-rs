use crate::{
    endpoint::{EndpointDirection, EndpointHandle, EndpointInfo},
    performer::{Endpoint, EndpointError, EndpointType, Performer},
    value::ValueRef,
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

impl EndpointType for InputEvent {
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

    fn handle(&self) -> EndpointHandle {
        self.handle
    }
}

impl EndpointType for OutputEvent {
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

    fn handle(&self) -> EndpointHandle {
        self.handle
    }
}

pub fn post_event(
    performer: &mut Performer,
    Endpoint(endpoint): Endpoint<InputEvent>,
    event: ValueRef<'_>,
) -> Result<(), EndpointError> {
    let type_index = performer
        .endpoints
        .get(&endpoint.handle)
        .ok_or(EndpointError::EndpointDoesNotExist)?
        .as_event()
        .ok_or(EndpointError::EndpointTypeMismatch)?
        .type_index(event.ty())
        .ok_or(EndpointError::DataTypeMismatch)?;

    event.with_bytes(|bytes| {
        performer
            .ptr
            .add_input_event(endpoint.handle, type_index, bytes);
    });

    Ok(())
}

pub fn fetch_events(
    performer: &Performer,
    Endpoint(endpoint): Endpoint<OutputEvent>,
    mut callback: impl FnMut(usize, ValueRef<'_>),
) -> Result<(), EndpointError> {
    let types = performer
        .endpoints
        .get(&endpoint.handle)
        .and_then(|endpoint| endpoint.as_event())
        .map(|endpoint| endpoint.types())
        .expect("endpoint should exist and be an event endpoint");

    performer
        .ptr
        .iterate_output_events(endpoint.handle, |frame_offset, _, type_index, data| {
            let ty = types.get(usize::from(type_index));
            debug_assert!(ty.is_some(), "Invalid type index from Cmajor");

            if let Some(ty) = ty {
                callback(frame_offset, ValueRef::new_from_slice(ty.as_ref(), data));
            }
        });

    Ok(())
}
