use open_graphene_sdk_core::TransactionHeader;

pub trait GrapheneOperationBuilderTypes {
    type Transaction;
    type Operation;
    type Asset;
    type AccountId: From<String>;
    type AssetId: From<String>;
    type LimitOrderId: From<String>;
    type FutureExtensions;

    fn asset(amount: i64, asset_id: Self::AssetId) -> Self::Asset;
    fn empty_extensions() -> Self::FutureExtensions;

    fn transaction(
        header: TransactionHeader,
        operations: Vec<Self::Operation>,
        extensions: Self::FutureExtensions,
    ) -> Self::Transaction;
}
