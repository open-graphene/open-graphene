use open_graphene_transport::GrapheneSession;

use super::account_history::AccountHistoryRequest;
use super::account_history_by_id::AccountHistoryByIdRequest;
use super::constants::{DEFAULT_ACCOUNT_HISTORY_LIMIT, DEFAULT_ACCOUNT_HISTORY_OFFSET};
use super::fill_order_history::{DEFAULT_FILL_ORDER_HISTORY_LIMIT, FillOrderHistoryRequest};
use super::market_history::MarketHistoryRequest;

pub struct HistoryApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> HistoryApi<'session> {
    pub fn account_history<S>(self, account_name_or_id: S) -> AccountHistoryRequest<'session>
    where
        S: Into<String>,
    {
        AccountHistoryRequest {
            session: self.session,
            account_name_or_id: account_name_or_id.into(),
            limit: DEFAULT_ACCOUNT_HISTORY_LIMIT,
            offset: DEFAULT_ACCOUNT_HISTORY_OFFSET,
        }
    }

    pub fn fill_order_history<B, Q>(self, base: B, quote: Q) -> FillOrderHistoryRequest<'session>
    where
        B: Into<String>,
        Q: Into<String>,
    {
        FillOrderHistoryRequest {
            session: self.session,
            base: base.into(),
            quote: quote.into(),
            limit: DEFAULT_FILL_ORDER_HISTORY_LIMIT,
        }
    }

    pub fn account_history_by_id<S>(self, account_id: S) -> AccountHistoryByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountHistoryByIdRequest {
            session: self.session,
            account_id: account_id.into(),
            limit: DEFAULT_ACCOUNT_HISTORY_LIMIT,
            offset: DEFAULT_ACCOUNT_HISTORY_OFFSET,
        }
    }

    /// OHLC price candles for the `base`/`quote` market: set `.range(start, end)` and an optional
    /// `.bucket_seconds(..)`, then `.get()`.
    pub fn get_market_history<S>(self, base: S, quote: S) -> MarketHistoryRequest<'session>
    where
        S: Into<String>,
    {
        MarketHistoryRequest::new(self.session, base, quote)
    }

    /// Recent trades (filled orders) on the `base`/`quote` market: optional `.limit(..)`, then `.get()`.
    pub fn get_fill_order_history<S>(self, base: S, quote: S) -> FillOrderHistoryRequest<'session>
    where
        S: Into<String>,
    {
        FillOrderHistoryRequest::new(self.session, base, quote)
    }
}
