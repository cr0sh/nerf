pub mod binance;
pub mod common;

/// A layer to wrap incoming HTTP requests with corresponding signer.
pub struct HttpSignLayer<Context>(Context);

impl<Context> HttpSignLayer<Context> {
    pub fn new(context: Context) -> Self {
        Self(context)
    }
}

impl<S, Context> tower::Layer<S> for HttpSignLayer<Context>
where
    Context: Clone,
{
    type Service = HttpSignService<S, Context>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpSignService {
            inner,
            context: self.0.clone(),
        }
    }
}

pub struct HttpSignService<S, Context> {
    inner: S,
    context: Context,
}

impl<S, Context, T> tower::Service<T> for HttpSignService<S, Context>
where
    S: tower::Service<<T::Signer as nerf::Signer<T>>::Wrapped>,
    Context: Clone,
    T: nerf::HttpRequest,
    T::Signer: nerf::Signer<T, Context = Context>,
{
    type Response = S::Response;

    type Error = S::Error;

    type Future = S::Future;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: T) -> Self::Future {
        self.inner.call(<T::Signer as nerf::Signer<T>>::wrap_signer(
            req,
            self.context.clone(),
        ))
    }
}

/// Defines conversion of requests/responses to [`common`] types.
trait FromExchangeRequest<R> {}
