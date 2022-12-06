use std::{sync::Arc, time::Duration};

use nerf::ReadyCall;
use tokio::{
    sync::{oneshot, Mutex, Notify},
    task::JoinHandle,
};
use tower_service::Service;
use tracing::{trace, trace_span, Instrument};

/// A background [`task`] that periodically fetches up-to-date information from the [`Service`].
///
/// [`task`]: tokio::task
pub struct Fetcher<T> {
    _handle: JoinHandle<()>,
    value: Arc<Mutex<Option<T>>>,
    notify: Arc<Notify>,
    abort: Option<oneshot::Sender<()>>,
}

impl<T: Send + 'static, E: Send + 'static> Fetcher<Result<T, E>> {
    /// Constructs a new [`Fetcher`] instance which invokes the request every period
    /// to the service.
    pub fn new<R, S>(request: R, mut service: S, period: Duration) -> Self
    where
        R: Clone + Send + 'static,
        S: Service<R, Response = T, Error = E> + Send + 'static,
        S::Future: Send,
    {
        let value = Arc::new(Mutex::new(None));
        let notify = Arc::new(Notify::new());
        let (tx, mut rx) = oneshot::channel();

        let handle = tokio::spawn({
            let value = Arc::clone(&value);
            let notify = Arc::clone(&notify);

            (async move {
                let mut ticker = tokio::time::interval(period);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                loop {
                    tokio::select! {
                        _ = ticker.tick() =>  {
                            let req = request.clone();
                            let result = service.ready_call(req).await;

                            *value.lock().await = Some(result);
                            notify.notify_one();
                        }
                        _ = &mut rx => {
                            trace!("fetcher is aborting");
                            return;
                        }
                    }
                }
            })
            .instrument(trace_span!("fetcher"))
        });

        Self {
            _handle: handle,
            value,
            notify,
            abort: Some(tx),
        }
    }
}

impl<T> Fetcher<T> {
    /// Returns an item fetched by the [`Fetcher`].
    /// If the item is already taken and the [`Fetcher`] did not fetch it again,
    /// waits until the next item is avilable.
    pub async fn next(&mut self) -> T {
        loop {
            self.notify.notified().await;
            if let Some(x) = self.value.lock().await.take() {
                return x;
            }
        }
    }

    /// Transforms the [`Fetcher`] instance into [`CachedFetcher`].
    pub fn cached(self) -> CachedFetcher<T>
    where
        T: Clone,
    {
        CachedFetcher {
            fetcher: self,
            cache: None,
        }
    }
}

impl<T> Drop for Fetcher<T> {
    fn drop(&mut self) {
        let _ = self.abort.take().unwrap().send(());
    }
}

/// A [`Fetcher`] that caches the last value fetched.
pub struct CachedFetcher<T> {
    fetcher: Fetcher<T>,
    cache: Option<T>,
}

impl<T: Clone> CachedFetcher<T> {
    /// Try to get a value from the inner [`Fetcher`].
    /// If value is not yet pulled, use the cached value from previous invocation.
    /// If cache is not available (first call), this method waits until the fetcher is run.
    pub async fn get(&mut self) -> T {
        if let Some(cached) = &self.cache {
            self.fetcher
                .value
                .lock()
                .await
                .take()
                .unwrap_or_else(|| cached.clone())
        } else {
            let v = self.fetcher.next().await;
            self.cache = Some(v.clone());
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, task::Poll};

    use futures::future::{ready, Ready};
    use tokio::time::Instant;
    use tower::Service;

    use super::*;

    struct TestService(u32);
    impl Service<u32> for TestService {
        type Response = u32;

        type Error = Infallible;

        type Future = Ready<Result<u32, Infallible>>;

        fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: u32) -> Self::Future {
            self.0 += req;
            ready(Ok(self.0))
        }
    }

    #[tokio::test(start_paused = true)]
    async fn trivial() {
        let mut fetcher = Fetcher::new(1, TestService(1), Duration::from_secs(1));
        assert_eq!(fetcher.next().await, Ok(2));
        assert_eq!(fetcher.next().await, Ok(3));
        assert_eq!(fetcher.next().await, Ok(4));
    }

    #[tokio::test(start_paused = true)]
    async fn timings() {
        let start = Instant::now();
        let mut fetcher = Fetcher::new(1, TestService(1), Duration::from_secs(1));
        assert_eq!(fetcher.next().await, Ok(2));
        assert_eq!(fetcher.next().await, Ok(3));
        assert_eq!(fetcher.next().await, Ok(4));
        let elapsed = start.elapsed();
        assert_eq!(elapsed, Duration::from_secs(2));
    }

    #[tokio::test(start_paused = true)]
    async fn missed_ticks() {
        let start = Instant::now();
        let mut fetcher = Fetcher::new(1, TestService(1), Duration::from_secs(1));
        tokio::time::sleep(Duration::from_millis(2100)).await;
        assert_eq!(fetcher.next().await, Ok(4));
        assert_eq!(fetcher.next().await, Ok(5));
        assert_eq!(fetcher.next().await, Ok(6));
        let elapsed = start.elapsed();
        assert_eq!(elapsed, Duration::from_secs(4));
    }
}
