use {
    crate::{
        engine::{endpoint::Endpoints, Endpoint, EndpointHandle},
        performer::{spsc, spsc::EndpointMessage},
        value::Value,
    },
    std::sync::Arc,
};

pub struct PerformerHandle {
    pub(super) endpoints: Arc<Endpoints>,
    pub(super) endpoint_tx: spsc::EndpointSender,
}

#[derive(Debug, thiserror::Error)]
pub enum EndpointError {
    #[error("no such endpoint")]
    EndpointDoesNotExist,

    #[error("type mismatch")]
    EndpointTypeMismatch,

    #[error("data type mismatch")]
    DataTypeMismatch,

    #[error("failed to send message to performer")]
    FailedToSendMessageToPerformer,
}

impl PerformerHandle {
    pub fn get_input(&self, id: impl AsRef<str>) -> Option<(EndpointHandle, &Endpoint)> {
        self.endpoints.get_input_by_id(id)
    }

    pub fn write_value(
        &mut self,
        handle: EndpointHandle,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get_input(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let endpoint = if let Endpoint::Value(value) = endpoint {
            value
        } else {
            return Err(EndpointError::EndpointTypeMismatch);
        };

        let value = value.into();

        if endpoint.ty().as_ref() != value.ty() {
            return Err(EndpointError::DataTypeMismatch);
        }

        value.with_bytes(|bytes| {
            let message = EndpointMessage::Value {
                handle,
                data: bytes,
                num_frames_to_reach_value: 0,
            };

            self.endpoint_tx
                .send(message)
                .map_err(|_| EndpointError::FailedToSendMessageToPerformer)
        })
    }

    pub fn post_event(
        &mut self,
        handle: EndpointHandle,
        value: impl Into<Value>,
    ) -> Result<(), EndpointError> {
        let endpoint = self
            .endpoints
            .get_input(handle)
            .ok_or(EndpointError::EndpointDoesNotExist)?;

        let endpoint = if let Endpoint::Event(endpoint) = endpoint {
            endpoint
        } else {
            return Err(EndpointError::EndpointTypeMismatch);
        };

        let value = value.into();

        let type_index = endpoint
            .type_index(value.ty())
            .ok_or(EndpointError::DataTypeMismatch)?;

        value.with_bytes(|bytes| {
            let message = EndpointMessage::Event {
                handle,
                type_index,
                data: bytes,
            };

            self.endpoint_tx
                .send(message)
                .map_err(|_| EndpointError::FailedToSendMessageToPerformer)
        })
    }
}
