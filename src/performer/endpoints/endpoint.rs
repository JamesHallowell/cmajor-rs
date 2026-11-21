use {
    super::EndpointKind,
    crate::{endpoint::EndpointHandle, engine::ProgramId, performer::Performer},
    std::{fmt, marker::PhantomData},
};

/// An endpoint of a program.
pub struct Endpoint<K: EndpointKind> {
    program: ProgramId,
    handle: EndpointHandle,
    binding: K::Binding,
    _kind: PhantomData<fn() -> K>,
}

impl<K: EndpointKind> Endpoint<K> {
    pub(crate) fn new(program: ProgramId, handle: EndpointHandle, binding: K::Binding) -> Self {
        Self {
            program,
            handle,
            binding,
            _kind: PhantomData,
        }
    }

    /// The handle to pass to `performer` for this endpoint.
    ///
    /// # Panics
    ///
    /// Panics if `performer` does not run the program this endpoint was bound against.
    pub(super) fn handle_on(&self, performer: &Performer) -> EndpointHandle {
        assert_eq!(
            self.program, performer.program,
            "endpoint was bound against a different program than this performer runs"
        );
        self.handle
    }

    pub(super) fn binding(&self) -> &K::Binding {
        &self.binding
    }
}

impl<K: EndpointKind> Clone for Endpoint<K> {
    fn clone(&self) -> Self {
        Self::new(self.program, self.handle, self.binding.clone())
    }
}

impl<K: EndpointKind> Copy for Endpoint<K> where K::Binding: Copy {}

impl<K: EndpointKind> fmt::Debug for Endpoint<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Endpoint")
            .field("program", &self.program)
            .field("handle", &self.handle)
            .field("binding", &self.binding)
            .finish()
    }
}
