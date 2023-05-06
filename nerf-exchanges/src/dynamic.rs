use std::{
    any::Any,
    error::Error,
    sync::{Arc, Mutex},
    task::Poll,
};

use nerf::ReadyCall;
use tower::{util::BoxService, ServiceExt};

use crate::common::{
    CancelAllOrders, CancelOrder, CommonOps, CommonOpsService, GetAllOrders, GetBalance,
    GetOrderbook, GetOrders, GetPosition, GetTickers, GetTrades, IntoMarket, Order, PlaceOrder,
};

/// A boxed [`CommonOpsService`].
/// Note that its [`tower::Service`] implementation does not offer backpressure. Its `poll_ready`
/// is a no-op and calls the actual `poll_ready` method in the `call` implementation.
/// Also, there is no error handling while converting `<R>` into `<CommonOps::R#Request>`: it
/// simply panics if the conversion fails.
pub struct BoxCommonOpsService {
    get_tickers: BoxService<
        GetTickers,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    get_trades: BoxService<
        GetTrades,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    get_orderbook: BoxService<
        GetOrderbook,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    get_orders: BoxService<
        GetOrders,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    get_all_orders: BoxService<
        GetAllOrders,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    place_order: BoxService<
        PlaceOrder,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    cancel_order: BoxService<
        CancelOrder,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    cancel_all_orders: BoxService<
        CancelAllOrders,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    get_balance: BoxService<
        GetBalance,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
    get_position: BoxService<
        GetPosition,
        Box<dyn Any + Send + 'static>,
        Box<dyn Error + Send + Sync + 'static>,
    >,
}

impl BoxCommonOpsService {
    /// Creates a new [`BoxCommonOpsService`] instance.
    pub fn new<T>(svc: T) -> Self
    where
        T: CommonOps + CommonOpsService + Send + 'static,
        <<T as CommonOps>::GetTickersRequest as std::convert::TryFrom<GetTickers>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetTickersRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetTickersRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetTickersRequest>>::Response: Send + 'static,
        <<T as CommonOps>::GetTradesRequest as std::convert::TryFrom<GetTrades>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetTradesRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetTradesRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetTradesRequest>>::Response: Send + 'static,
        <<T as CommonOps>::GetOrderbookRequest as std::convert::TryFrom<GetOrderbook>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetOrderbookRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetOrderbookRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetOrderbookRequest>>::Response: Send + 'static,
        <<T as CommonOps>::GetOrdersRequest as std::convert::TryFrom<GetOrders>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetOrdersRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetOrdersRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetOrdersRequest>>::Response: Send + 'static,
        <<T as CommonOps>::GetAllOrdersRequest as std::convert::TryFrom<GetAllOrders>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetAllOrdersRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetAllOrdersRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetAllOrdersRequest>>::Response: Send + 'static,
        <<T as CommonOps>::PlaceOrderRequest as std::convert::TryFrom<PlaceOrder>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::PlaceOrderRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::PlaceOrderRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::PlaceOrderRequest>>::Response: Send + 'static,
        <<T as CommonOps>::CancelOrderRequest as std::convert::TryFrom<CancelOrder>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::CancelOrderRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::CancelOrderRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::CancelOrderRequest>>::Response: Send + 'static,
        <<T as CommonOps>::CancelAllOrdersRequest as std::convert::TryFrom<CancelAllOrders>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::CancelAllOrdersRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::CancelAllOrdersRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::CancelAllOrdersRequest>>::Response: Send + 'static,
        <<T as CommonOps>::GetBalanceRequest as std::convert::TryFrom<GetBalance>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetBalanceRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetBalanceRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetBalanceRequest>>::Response: Send + 'static,
        <<T as CommonOps>::GetPositionRequest as std::convert::TryFrom<GetPosition>>::Error:
            std::fmt::Debug,
        <T as tower::Service<<T as CommonOps>::GetPositionRequest>>::Error:
            Error + Send + Sync + 'static,
        <T as tower::Service<<T as CommonOps>::GetPositionRequest>>::Future: Send + 'static,
        <T as tower::Service<<T as CommonOps>::GetPositionRequest>>::Response: Send + 'static,
    {
        let arc_mutex = Arc::new(Mutex::new(svc));
        let get_tickers = tower::ServiceExt::<GetTickers>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetTickers| {
                    <T as CommonOps>::GetTickersRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let get_trades = tower::ServiceExt::<GetTrades>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetTrades| {
                    <T as CommonOps>::GetTradesRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let get_orderbook = tower::ServiceExt::<GetOrderbook>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetOrderbook| {
                    <T as CommonOps>::GetOrderbookRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let get_orders = tower::ServiceExt::<GetOrders>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetOrders| {
                    <T as CommonOps>::GetOrdersRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let get_all_orders = tower::ServiceExt::<GetAllOrders>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetAllOrders| {
                    <T as CommonOps>::GetAllOrdersRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let place_order = tower::ServiceExt::<PlaceOrder>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: PlaceOrder| {
                    <T as CommonOps>::PlaceOrderRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let cancel_order = tower::ServiceExt::<CancelOrder>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: CancelOrder| {
                    <T as CommonOps>::CancelOrderRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let cancel_all_orders = tower::ServiceExt::<CancelAllOrders>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: CancelAllOrders| {
                    <T as CommonOps>::CancelAllOrdersRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let get_balance = tower::ServiceExt::<GetBalance>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetBalance| {
                    <T as CommonOps>::GetBalanceRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        let get_position = tower::ServiceExt::<GetPosition>::boxed(
            ArcMutexService(Arc::clone(&arc_mutex))
                .map_request(|x: GetPosition| {
                    <T as CommonOps>::GetPositionRequest::try_from(x)
                        .expect("cannot convert a generic request to an associated type")
                })
                .map_result(|res| match res {
                    Ok(x) => Ok(Box::new(x) as Box<dyn Any + Send + 'static>),
                    Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync + 'static>),
                }),
        );
        BoxCommonOpsService {
            get_tickers,
            get_trades,
            get_orderbook,
            get_orders,
            get_all_orders,
            place_order,
            cancel_order,
            cancel_all_orders,
            get_balance,
            get_position,
        }
    }

    pub async fn get_tickers(
        &mut self,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        self.get_tickers.ready_call(GetTickers).await
    }

    pub async fn get_trades(
        &mut self,
        market: impl IntoMarket,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        let market = market.into_market();
        self.get_trades.ready_call(GetTrades { market }).await
    }

    pub async fn get_orderbook(
        &mut self,
        market: impl IntoMarket,
        ticks: Option<u64>,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        let market = market.into_market();
        self.get_orderbook
            .ready_call(GetOrderbook { market, ticks })
            .await
    }

    pub async fn get_orders(
        &mut self,
        market: impl IntoMarket,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        let market = market.into_market();
        self.get_orders.ready_call(GetOrders { market }).await
    }

    pub async fn get_all_orders(
        &mut self,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        self.get_all_orders.ready_call(GetAllOrders).await
    }

    pub async fn place_order(
        &mut self,
        market: impl IntoMarket,
        order: Order,
        reduce_only: bool,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        let market = market.into_market();
        self.place_order
            .ready_call(PlaceOrder {
                market,
                order,
                reduce_only,
            })
            .await
    }

    pub async fn cancel_order(
        &mut self,
        market: impl IntoMarket,
        order_id: String,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        let market = market.into_market();
        self.cancel_order
            .ready_call(CancelOrder { market, order_id })
            .await
    }

    pub async fn cancel_all_orders(
        &mut self,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        self.cancel_all_orders.ready_call(CancelAllOrders).await
    }

    pub async fn get_balance(
        &mut self,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        self.get_balance.ready_call(GetBalance).await
    }

    pub async fn get_position(
        &mut self,
        market: impl IntoMarket,
    ) -> Result<Box<dyn Any + Send + 'static>, Box<dyn Error + Send + Sync + 'static>> {
        let market = market.into_market();
        self.get_position.ready_call(GetPosition { market }).await
    }
}

pub struct ArcMutexService<T>(Arc<Mutex<T>>);

impl<T, S> tower::Service<S> for ArcMutexService<T>
where
    T: tower::Service<S>,
{
    type Response = T::Response;

    type Error = T::Error;

    type Future = T::Future;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.0.lock().unwrap().poll_ready(cx)
    }

    fn call(&mut self, req: S) -> Self::Future {
        self.0.lock().unwrap().call(req)
    }
}
