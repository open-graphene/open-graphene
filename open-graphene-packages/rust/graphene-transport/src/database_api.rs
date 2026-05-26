use serde_json::{json, Value};

use crate::{GrapheneSession, TransportError};

pub fn get_objects<I, S>(session: &mut GrapheneSession, ids: I) -> Result<Value, TransportError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    session.database_call("get_objects", get_objects_params(ids))
}

pub fn get_required_fees(
    session: &mut GrapheneSession,
    operations_json: Value,
    fee_asset_id: impl Into<String>,
) -> Result<Value, TransportError> {
    session.database_call(
        "get_required_fees",
        get_required_fees_params(operations_json, fee_asset_id),
    )
}

pub fn lookup_accounts(
    session: &mut GrapheneSession,
    lower_bound_name: impl Into<String>,
    limit: u64,
) -> Result<Value, TransportError> {
    session.database_call(
        "lookup_accounts",
        lookup_accounts_params(lower_bound_name, limit),
    )
}

pub fn lookup_asset_symbols<I, S>(
    session: &mut GrapheneSession,
    symbols: I,
) -> Result<Value, TransportError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    session.database_call("lookup_asset_symbols", lookup_asset_symbols_params(symbols))
}

fn get_objects_params<I, S>(ids: I) -> Value
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let ids = ids.into_iter().map(Into::into).collect::<Vec<String>>();
    json!([ids])
}

fn get_required_fees_params(operations_json: Value, fee_asset_id: impl Into<String>) -> Value {
    json!([operations_json, fee_asset_id.into()])
}

fn lookup_accounts_params(lower_bound_name: impl Into<String>, limit: u64) -> Value {
    json!([lower_bound_name.into(), limit])
}

fn lookup_asset_symbols_params<I, S>(symbols: I) -> Value
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let symbols = symbols.into_iter().map(Into::into).collect::<Vec<String>>();
    json!([symbols])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_object_ids_in_graphene_params_array() {
        assert_eq!(
            get_objects_params(["2.1.0", "1.2.100"]),
            json!([["2.1.0", "1.2.100"]])
        );
    }

    #[test]
    fn preserves_empty_object_id_list_shape() {
        assert_eq!(get_objects_params(Vec::<String>::new()), json!([[]]));
    }

    #[test]
    fn wraps_required_fees_operations_and_asset_id_in_graphene_params_array() {
        assert_eq!(
            get_required_fees_params(json!([[0, {"foo": "bar"}]]), "1.3.0"),
            json!([[[0, {"foo": "bar"}]], "1.3.0"])
        );
    }

    #[test]
    fn builds_lookup_accounts_params() {
        assert_eq!(
            lookup_accounts_params("swaplock", 1),
            json!(["swaplock", 1])
        );
    }

    #[test]
    fn wraps_asset_symbols_in_graphene_params_array() {
        assert_eq!(
            lookup_asset_symbols_params(["TEST", "BTS"]),
            json!([["TEST", "BTS"]])
        );
    }
}
