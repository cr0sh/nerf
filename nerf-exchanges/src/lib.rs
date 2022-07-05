pub mod binance;

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

/// Defines a [`tower::Layer`], [`tower::Service`], and dedicated [`std::error::Error`] and
/// [`std::future::Future`] implementor to bridge between exchange-local request types
/// and nerf backends like [`hyper::Client`].
///
/// Input is quite smiple: identifiers to define the layer, service, error, and future type.
///
/// # Example
///
/// ```rust
/// define_layer!(MyExchangeLayer, MyExchangeService, MyExchangeError)
/// ```
#[macro_export]
macro_rules! define_layer {
    ($layer:ident, $service:ident, $error:ident, $future:ident) => {
        pub struct Request<T>(T);

        impl<T> ::nerf::Request for Request<T>
        where
            T: ::nerf::Request,
        {
            type Response = Response<T::Response>;
        }

        impl<T> ::nerf::HttpRequest for Request<T>
        where
            T: ::nerf::HttpRequest,
        {
            type Signer = T::Signer;

            fn method(&self) -> ::hyper::http::Method {
                self.0.method()
            }

            fn uri(&self) -> ::hyper::http::Uri {
                self.0.uri()
            }
        }

        pub struct Response<T>(T);

        #[derive(Debug)]
        pub enum $error<E1, E2> {
            Local(E1),
            Remote(E2),
        }

        impl<E1, E2> ::std::fmt::Display for $error<E1, E2>
        where
            E1: ::std::fmt::Display,
            E2: ::std::fmt::Display,
        {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    $error::Local(x) => x.fmt(f),
                    $error::Remote(x) => x.fmt(f),
                }
            }
        }

        impl<E1, E2> ::std::error::Error for $error<E1, E2>
        where
            E1: ::std::error::Error,
            E2: ::std::error::Error,
        {
            fn source(&self) -> Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    $error::Local(x) => x.source(),
                    $error::Remote(x) => x.source(),
                }
            }

            fn cause(&self) -> Option<&dyn ::std::error::Error> {
                self.source()
            }
        }

        pub struct $layer(());

        impl $layer {
            pub fn new() -> Self {
                Self(())
            }
        }

        impl Default for $layer {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<S> tower::Layer<S> for $layer {
            type Service = $service<S>;

            fn layer(&self, inner: S) -> Self::Service {
                $service { inner }
            }
        }

        pub struct $service<S> {
            inner: S,
        }

        impl<S, R> tower::Service<R> for $service<S>
        where
            S: tower::Service<Request<R>, Response = Response<R::Response>>,
            R: nerf::Request,
        {
            type Response = R::Response;

            type Error = S::Error;

            type Future = $future<S::Future, R::Response, S::Error>;

            fn poll_ready(
                &mut self,
                cx: &mut ::std::task::Context<'_>,
            ) -> ::std::task::Poll<Result<(), Self::Error>> {
                self.inner.poll_ready(cx)
            }

            fn call(&mut self, req: R) -> Self::Future {
                $future(self.inner.call(Request(req)), ::std::marker::PhantomData)
            }
        }

        #[pin_project]
        pub struct $future<F, T, E>(#[pin] F, ::std::marker::PhantomData<(T, E)>);

        impl<F, T, E> ::std::future::Future for $future<F, T, E>
        where
            F: ::std::future::Future<Output = ::std::result::Result<Response<T>, E>>,
        {
            type Output = Result<T, E>;

            fn poll(
                self: ::std::pin::Pin<&mut Self>,
                cx: &mut ::std::task::Context<'_>,
            ) -> ::std::task::Poll<Self::Output> {
                self.project().0.poll(cx).map(|x| x.map(|Response(x)| x))
            }
        }
    };
}
