use open_graphene_transport::GrapheneSession;

use super::grouped_limit_orders::{DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT, GroupedLimitOrdersRequest};
use super::tracked_groups::TrackedGroupsRequest;

/// Entry point for the Swaplock `orders` API (grouped market order book).
pub struct OrdersApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> OrdersApi<'session> {
    /// The group widths the node tracks (in 0.01% units).
    pub fn tracked_groups(self) -> TrackedGroupsRequest<'session> {
        TrackedGroupsRequest {
            session: self.session,
        }
    }

    /// Grouped limit orders for the `base`/`quote` market at the given `group` width.
    pub fn grouped_limit_orders(
        self,
        base_asset: impl Into<String>,
        quote_asset: impl Into<String>,
        group: u16,
    ) -> GroupedLimitOrdersRequest<'session> {
        GroupedLimitOrdersRequest {
            session: self.session,
            base_asset: base_asset.into(),
            quote_asset: quote_asset.into(),
            group,
            start: None,
            limit: DEFAULT_GROUPED_LIMIT_ORDERS_LIMIT,
        }
    }
}
