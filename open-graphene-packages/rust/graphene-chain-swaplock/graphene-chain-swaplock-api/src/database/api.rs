use graphene_chain_swaplock_bindings::generated::ids::AccountId;
use graphene_chain_swaplock_bindings::generated::rpc::ProtocolObject;
use graphene_chain_swaplock_bindings::generated::rpc::database::get_config::Config;
use graphene_chain_swaplock_bindings::generated::types::{MaybeSignedBlockHeader, SignedBlock};
use graphene_chain_swaplock_bindings::generated::{
    AccountObject, Asset, AssetObject, ChainPropertyObject, DynamicGlobalPropertyObject,
    GlobalPropertyObject, LimitOrderObject,
};
use open_graphene_transport::GrapheneSession;

use crate::SwaplockApiError;

use super::account_balances::AccountBalancesRequest;
use super::account_balances_by_id::{AccountBalancesByIdRequest, get_account_balances_by_id};
use super::account_by_id::{AccountByIdRequest, get_account_by_id};
use super::account_by_name::{AccountByNameRequest, get_account_by_name};
use super::account_orders::AccountOrdersRequest;
use super::account_orders_by_id::{AccountOrdersByIdRequest, get_account_orders_by_id};
use super::accounts::AccountsRequest;
use super::asset_by_id::{AssetByIdRequest, get_asset_by_id};
use super::asset_by_symbol::{AssetBySymbolRequest, get_asset_by_symbol};
use super::chain_id::{ChainIdRequest, get_chain_id};
use super::chain_properties::{ChainPropertiesRequest, get_chain_properties};
use super::content_card_by_id::ContentCardByIdRequest;
use super::content_cards_by_author::ContentCardsByAuthorRequest;
use super::content_cards_by_room::ContentCardsByRoomRequest;
use super::data_room_by_id::DataRoomByIdRequest;
use super::data_room_key_epoch::DataRoomKeyEpochRequest;
use super::data_room_key_epochs::DataRoomKeyEpochsRequest;
use super::data_room_member::DataRoomMemberRequest;
use super::data_room_members::DataRoomMembersRequest;
use super::data_rooms_by_member::DataRoomsByMemberRequest;
use super::data_rooms_by_owner::DataRoomsByOwnerRequest;
use super::data_rooms_by_subject::DataRoomsBySubjectRequest;
use super::dynamic_global_properties::{
    DynamicGlobalPropertiesRequest, DynamicGlobalPropertiesSubscription,
    get_dynamic_global_properties,
};
use super::get_block::GetBlockRequest;
use super::get_block_header::GetBlockHeaderRequest;
use super::get_config::GetConfigRequest;
use super::get_key_references::{GetKeyReferencesRequest, get_key_references_request};
use super::get_limit_orders::{DEFAULT_GET_LIMIT_ORDERS_LIMIT, GetLimitOrdersRequest};
use super::get_objects::{GetObjectsRequest, get_objects_request};
use super::get_ticker::GetTickerRequest;
use super::global_properties::{GlobalPropertiesRequest, get_global_properties};
use super::list_assets::{DEFAULT_LIST_ASSETS_LIMIT, ListAssetsRequest};
use super::lookup_accounts::{DEFAULT_LOOKUP_ACCOUNTS_LIMIT, LookupAccountsRequest};
use super::proposed_transactions::ProposedTransactionsRequest;
use super::string_list::IntoStringList;

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> DatabaseApi<'session> {
    pub fn account_balances<A, L>(
        self,
        account_name: A,
        asset_symbols: L,
    ) -> AccountBalancesRequest<'session>
    where
        A: Into<String>,
        L: IntoStringList,
    {
        AccountBalancesRequest {
            session: self.session,
            account_name: account_name.into(),
            asset_symbols: asset_symbols.into_string_list(),
        }
    }

    pub fn account_balances_by_id<A, L>(
        self,
        account_id: A,
        asset_ids: L,
    ) -> AccountBalancesByIdRequest<'session>
    where
        A: Into<String>,
        L: IntoStringList,
    {
        AccountBalancesByIdRequest {
            session: self.session,
            account_id: account_id.into(),
            asset_ids: asset_ids.into_string_list(),
        }
    }

    pub fn account_orders<S>(self, account_name: S) -> AccountOrdersRequest<'session>
    where
        S: Into<String>,
    {
        AccountOrdersRequest {
            session: self.session,
            account_name: account_name.into(),
        }
    }

    pub fn account_orders_by_id<S>(self, account_id: S) -> AccountOrdersByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountOrdersByIdRequest {
            session: self.session,
            account_id: account_id.into(),
        }
    }

    pub fn account_by_id<S>(self, account_id: S) -> AccountByIdRequest<'session>
    where
        S: Into<String>,
    {
        AccountByIdRequest {
            session: self.session,
            account_id: account_id.into(),
        }
    }

    pub fn account_by_name<S>(self, account_name: S) -> AccountByNameRequest<'session>
    where
        S: Into<String>,
    {
        AccountByNameRequest {
            session: self.session,
            account_name: account_name.into(),
        }
    }

    pub fn accounts<I, S>(self, names_or_ids: I) -> AccountsRequest<'session>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        AccountsRequest {
            session: self.session,
            names_or_ids: names_or_ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn asset_by_id<S>(self, asset_id: S) -> AssetByIdRequest<'session>
    where
        S: Into<String>,
    {
        AssetByIdRequest {
            session: self.session,
            asset_id: asset_id.into(),
        }
    }

    pub fn asset_by_symbol<S>(self, asset_symbol: S) -> AssetBySymbolRequest<'session>
    where
        S: Into<String>,
    {
        AssetBySymbolRequest {
            session: self.session,
            asset_symbol: asset_symbol.into(),
        }
    }

    pub fn chain_id(self) -> ChainIdRequest<'session> {
        ChainIdRequest {
            session: self.session,
        }
    }

    pub fn chain_properties(self) -> ChainPropertiesRequest<'session> {
        ChainPropertiesRequest {
            session: self.session,
        }
    }

    /// Fetch one content card by its id (like `1.10.0`); `None` for an unknown id.
    pub fn content_card_by_id<S>(self, content_id: S) -> ContentCardByIdRequest<'session>
    where
        S: Into<String>,
    {
        ContentCardByIdRequest {
            session: self.session,
            content_id: content_id.into(),
        }
    }

    /// The content cards authored by an account (name or id), pageable with
    /// `.limit(..)` / `.start_id(..)`.
    pub fn content_cards_by_author<S>(
        self,
        account_name_or_id: S,
    ) -> ContentCardsByAuthorRequest<'session>
    where
        S: Into<String>,
    {
        ContentCardsByAuthorRequest {
            session: self.session,
            account_name_or_id: account_name_or_id.into(),
            limit: None,
            start_id: None,
        }
    }

    /// The content cards of a data room (id like `1.9.0`), pageable with
    /// `.limit(..)` / `.start_id(..)`.
    pub fn content_cards_by_room<S>(self, room_id: S) -> ContentCardsByRoomRequest<'session>
    where
        S: Into<String>,
    {
        ContentCardsByRoomRequest {
            session: self.session,
            room_id: room_id.into(),
            limit: None,
            start_id: None,
        }
    }

    /// Fetch one data room by its id (like `1.9.0`); `None` for an unknown id.
    pub fn data_room_by_id<S>(self, room_id: S) -> DataRoomByIdRequest<'session>
    where
        S: Into<String>,
    {
        DataRoomByIdRequest {
            session: self.session,
            room_id: room_id.into(),
        }
    }

    /// Live proposals relevant to an account: proposed by it or awaiting its approval.
    pub fn proposed_transactions<S>(
        self,
        account_name_or_id: S,
    ) -> ProposedTransactionsRequest<'session>
    where
        S: Into<String>,
    {
        ProposedTransactionsRequest {
            session: self.session,
            account_name_or_id: account_name_or_id.into(),
        }
    }

    /// One account's key record for a given epoch of a data room; `None` when it does not exist.
    pub fn data_room_key_epoch<R, A>(
        self,
        room_id: R,
        epoch: u32,
        account_name_or_id: A,
    ) -> DataRoomKeyEpochRequest<'session>
    where
        R: Into<String>,
        A: Into<String>,
    {
        DataRoomKeyEpochRequest {
            session: self.session,
            room_id: room_id.into(),
            epoch,
            account_name_or_id: account_name_or_id.into(),
        }
    }

    /// One account's key epoch records in a data room, pageable with
    /// `.limit(..)` / `.start_id(..)`.
    pub fn data_room_key_epochs<R, A>(
        self,
        room_id: R,
        account_name_or_id: A,
    ) -> DataRoomKeyEpochsRequest<'session>
    where
        R: Into<String>,
        A: Into<String>,
    {
        DataRoomKeyEpochsRequest {
            session: self.session,
            room_id: room_id.into(),
            account_name_or_id: account_name_or_id.into(),
            limit: None,
            start_id: None,
        }
    }

    /// One account's member record in a data room; `None` when the account is not a member.
    pub fn data_room_member<R, A>(
        self,
        room_id: R,
        account_name_or_id: A,
    ) -> DataRoomMemberRequest<'session>
    where
        R: Into<String>,
        A: Into<String>,
    {
        DataRoomMemberRequest {
            session: self.session,
            room_id: room_id.into(),
            account_name_or_id: account_name_or_id.into(),
        }
    }

    /// The member records of a data room (id like `1.9.0`), pageable with
    /// `.limit(..)` / `.start_id(..)`.
    pub fn data_room_members<S>(self, room_id: S) -> DataRoomMembersRequest<'session>
    where
        S: Into<String>,
    {
        DataRoomMembersRequest {
            session: self.session,
            room_id: room_id.into(),
            limit: None,
            start_id: None,
        }
    }

    /// The data room memberships of an account (name or id), pageable with
    /// `.limit(..)` / `.start_id(..)`; each member record carries the room id inside.
    pub fn data_rooms_by_member<S>(
        self,
        account_name_or_id: S,
    ) -> DataRoomsByMemberRequest<'session>
    where
        S: Into<String>,
    {
        DataRoomsByMemberRequest {
            session: self.session,
            account_name_or_id: account_name_or_id.into(),
            limit: None,
            start_id: None,
        }
    }

    /// The data rooms owned by an account (name or id), pageable with
    /// `.limit(..)` / `.start_id(..)`.
    pub fn data_rooms_by_owner<S>(self, account_name_or_id: S) -> DataRoomsByOwnerRequest<'session>
    where
        S: Into<String>,
    {
        DataRoomsByOwnerRequest {
            session: self.session,
            account_name_or_id: account_name_or_id.into(),
            limit: None,
            start_id: None,
        }
    }

    /// The data rooms attached to a subject: an asset symbol or id, or an account name or id.
    /// Pageable with `.limit(..)` / `.start_id(..)`.
    pub fn data_rooms_by_subject<S>(
        self,
        asset_or_account: S,
    ) -> DataRoomsBySubjectRequest<'session>
    where
        S: Into<String>,
    {
        DataRoomsBySubjectRequest {
            session: self.session,
            asset_or_account: asset_or_account.into(),
            limit: None,
            start_id: None,
        }
    }

    pub fn dynamic_global_properties(self) -> DynamicGlobalPropertiesRequest<'session> {
        DynamicGlobalPropertiesRequest {
            session: self.session,
        }
    }

    pub fn global_properties(self) -> GlobalPropertiesRequest<'session> {
        GlobalPropertiesRequest {
            session: self.session,
        }
    }

    /// List assets in symbol order, starting from `lower_bound` (`""` for the start).
    pub fn list_assets<S>(self, lower_bound: S) -> ListAssetsRequest<'session>
    where
        S: Into<String>,
    {
        ListAssetsRequest {
            session: self.session,
            lower_bound: lower_bound.into(),
            limit: DEFAULT_LIST_ASSETS_LIMIT,
        }
    }

    /// Look up account `(name, id)` pairs in name order, starting from `lower_bound`.
    pub fn lookup_accounts<S>(self, lower_bound: S) -> LookupAccountsRequest<'session>
    where
        S: Into<String>,
    {
        LookupAccountsRequest {
            session: self.session,
            lower_bound: lower_bound.into(),
            limit: DEFAULT_LOOKUP_ACCOUNTS_LIMIT,
        }
    }

    /// The raw order book for the `base`/`quote` market (asset ids like `1.3.0`).
    pub fn limit_orders<B, Q>(self, base: B, quote: Q) -> GetLimitOrdersRequest<'session>
    where
        B: Into<String>,
        Q: Into<String>,
    {
        GetLimitOrdersRequest {
            session: self.session,
            base: base.into(),
            quote: quote.into(),
            limit: DEFAULT_GET_LIMIT_ORDERS_LIMIT,
        }
    }

    /// Rolling ticker stats for the `base`/`quote` market (asset ids like `1.3.0`).
    pub fn ticker<B, Q>(self, base: B, quote: Q) -> GetTickerRequest<'session>
    where
        B: Into<String>,
        Q: Into<String>,
    {
        GetTickerRequest {
            session: self.session,
            base: base.into(),
            quote: quote.into(),
        }
    }

    /// Fetch a produced block by height (`None` if the chain has not reached it yet).
    pub fn block(self, block_num: u32) -> GetBlockRequest<'session> {
        GetBlockRequest {
            session: self.session,
            block_num,
        }
    }

    /// Fetch just a block's header by height, without its transactions.
    pub fn block_header(self, block_num: u32) -> GetBlockHeaderRequest<'session> {
        GetBlockHeaderRequest {
            session: self.session,
            block_num,
        }
    }

    /// Fetch the chain's compile-time constants (the `GRAPHENE_*` config parameters).
    pub fn config(self) -> GetConfigRequest<'session> {
        GetConfigRequest {
            session: self.session,
        }
    }

    /// Which accounts reference each public key in their authorities; one id list per key, in order.
    pub fn key_references<L>(self, keys: L) -> GetKeyReferencesRequest<'session>
    where
        L: IntoStringList,
    {
        get_key_references_request(self.session, keys)
    }

    /// Fetch any chain objects by id as raw JSON, e.g. `["2.1.0", "1.3.0"]`. The generic getter
    /// behind the typed ones; results keep the id order and unknown ids come back as `null`.
    pub fn objects<L>(self, ids: L) -> GetObjectsRequest<'session>
    where
        L: IntoStringList,
    {
        get_objects_request(self.session, ids)
    }

    pub async fn get_account_balances<L>(
        &mut self,
        account_name: &str,
        asset_symbols: L,
    ) -> Result<Vec<Asset>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        let account = get_account_by_name(self.session, account_name).await?;
        let mut asset_ids = Vec::new();
        for symbol in asset_symbols.into_string_list() {
            let asset = get_asset_by_symbol(self.session, &symbol).await?;
            asset_ids.push(asset.id.0);
        }
        get_account_balances_by_id(self.session, &account.id.0, asset_ids).await
    }

    pub async fn get_account_balances_by_id<L>(
        &mut self,
        account_id: &str,
        asset_ids: L,
    ) -> Result<Vec<Asset>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        get_account_balances_by_id(self.session, account_id, asset_ids.into_string_list()).await
    }

    pub async fn get_account_orders(
        &mut self,
        account_name: &str,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        let account = get_account_by_name(self.session, account_name).await?;
        get_account_orders_by_id(self.session, &account.id.0).await
    }

    pub async fn get_account_orders_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        get_account_orders_by_id(self.session, account_id).await
    }

    pub async fn get_limit_orders(
        &mut self,
        base: &str,
        quote: &str,
    ) -> Result<Vec<LimitOrderObject>, SwaplockApiError> {
        GetLimitOrdersRequest {
            session: &mut *self.session,
            base: base.to_string(),
            quote: quote.to_string(),
            limit: DEFAULT_GET_LIMIT_ORDERS_LIMIT,
        }
        .get()
        .await
    }

    pub async fn get_block(
        &mut self,
        block_num: u32,
    ) -> Result<Option<SignedBlock>, SwaplockApiError> {
        GetBlockRequest {
            session: &mut *self.session,
            block_num,
        }
        .get()
        .await
    }

    pub async fn get_block_header(
        &mut self,
        block_num: u32,
    ) -> Result<Option<MaybeSignedBlockHeader>, SwaplockApiError> {
        GetBlockHeaderRequest {
            session: &mut *self.session,
            block_num,
        }
        .get()
        .await
    }

    pub async fn get_config(&mut self) -> Result<Config, SwaplockApiError> {
        GetConfigRequest {
            session: &mut *self.session,
        }
        .get()
        .await
    }

    pub async fn get_objects<L>(
        &mut self,
        ids: L,
    ) -> Result<Vec<Option<ProtocolObject>>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        get_objects_request(self.session, ids).get().await
    }

    pub async fn get_key_references<L>(
        &mut self,
        keys: L,
    ) -> Result<Vec<Vec<AccountId>>, SwaplockApiError>
    where
        L: IntoStringList,
    {
        get_key_references_request(self.session, keys).get().await
    }

    pub async fn get_account_by_name(
        &mut self,
        account_name: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_name(self.session, account_name).await
    }

    pub async fn get_account_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        get_account_by_id(self.session, account_id).await
    }

    pub async fn get_asset_by_symbol(
        &mut self,
        asset_symbol: &str,
    ) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_symbol(self.session, asset_symbol).await
    }

    pub async fn get_asset_by_id(
        &mut self,
        asset_id: &str,
    ) -> Result<AssetObject, SwaplockApiError> {
        get_asset_by_id(self.session, asset_id).await
    }

    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        get_chain_id(self.session).await
    }

    pub async fn get_chain_properties(&mut self) -> Result<ChainPropertyObject, SwaplockApiError> {
        get_chain_properties(self.session).await
    }

    pub async fn get_dynamic_global_properties(
        &mut self,
    ) -> Result<DynamicGlobalPropertyObject, SwaplockApiError> {
        get_dynamic_global_properties(self.session).await
    }

    pub async fn get_global_properties(
        &mut self,
    ) -> Result<GlobalPropertyObject, SwaplockApiError> {
        get_global_properties(self.session).await
    }

    pub async fn subscribe_dynamic_global_properties(
        self,
    ) -> Result<DynamicGlobalPropertiesSubscription<'session>, SwaplockApiError> {
        self.dynamic_global_properties().subscribe().await
    }
}
