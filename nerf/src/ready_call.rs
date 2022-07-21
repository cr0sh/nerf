use tower::{util::Oneshot, Service, ServiceExt};

/// A trait providing [`ReadyCall::ready_call`] to [`Service`]s.
pub trait ReadyCall<Request>: Service<Request> + Sized {
    /// Calls the service when the service is ready to process the request.
    fn ready_call(&mut self, req: Request) -> Oneshot<&mut Self, Request> {
        self.oneshot(req)
    }
}

impl<T, Request> ReadyCall<Request> for T where T: Service<Request> {}
