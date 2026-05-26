use graphene_chain_swaplock_bindings::generated::AccountObject;
use open_graphene_transport::{GrapheneSession, get_objects, lookup_accounts, parse_chain_id};
use serde_json::{Value, json};

use crate::SwaplockApiError;

pub struct DatabaseApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl DatabaseApi<'_> {
    pub async fn get_chain_id(&mut self) -> Result<String, SwaplockApiError> {
        let value = self.session.database_call("get_chain_id", json!([]))?;
        Ok(parse_chain_id(value)?)
    }

    pub async fn get_account_by_name(
        &mut self,
        account_name: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        let account_id = lookup_account_id(self.session, account_name)?;
        self.get_account_by_id(&account_id).await
    }

    pub async fn get_account_by_id(
        &mut self,
        account_id: &str,
    ) -> Result<AccountObject, SwaplockApiError> {
        let value = get_objects(self.session, [account_id])?;
        account_object_from_get_objects(value, account_id)
    }
}

fn lookup_account_id(
    session: &mut GrapheneSession,
    account_name: &str,
) -> Result<String, SwaplockApiError> {
    let value = lookup_accounts(session, account_name, 1)?;
    let rows = value
        .as_array()
        .ok_or_else(|| unexpected_response("lookup_accounts", "expected array response"))?;

    for row in rows {
        let row = row
            .as_array()
            .ok_or_else(|| unexpected_response("lookup_accounts", "expected [name, id] row"))?;
        if row.len() != 2 {
            return Err(unexpected_response(
                "lookup_accounts",
                "expected [name, id] row with exactly two values",
            ));
        }
        let name = row[0]
            .as_str()
            .ok_or_else(|| unexpected_response("lookup_accounts", "account name must be string"))?;
        let id = row[1]
            .as_str()
            .ok_or_else(|| unexpected_response("lookup_accounts", "account id must be string"))?;
        if name == account_name {
            return Ok(id.to_string());
        }
    }

    Err(SwaplockApiError::AccountNotFound {
        account: account_name.to_string(),
    })
}

fn account_object_from_get_objects(
    value: Value,
    expected_account_id: &str,
) -> Result<AccountObject, SwaplockApiError> {
    let mut slots = value
        .as_array()
        .ok_or_else(|| unexpected_response("get_objects", "expected array response"))?
        .iter();
    let slot = slots
        .next()
        .ok_or_else(|| unexpected_response("get_objects", "expected one result slot"))?;
    if slots.next().is_some() {
        return Err(unexpected_response(
            "get_objects",
            "expected exactly one result slot",
        ));
    }
    if slot.is_null() {
        return Err(SwaplockApiError::AccountNotFound {
            account: expected_account_id.to_string(),
        });
    }

    let account: AccountObject = serde_json::from_value(slot.clone()).map_err(|error| {
        SwaplockApiError::UnexpectedResponse {
            method: "get_objects",
            message: error.to_string(),
        }
    })?;
    if account.id.0 != expected_account_id {
        return Err(unexpected_response(
            "get_objects",
            format!(
                "expected account id `{expected_account_id}`, got `{}`",
                account.id.0
            ),
        ));
    }

    Ok(account)
}

fn unexpected_response(method: &'static str, message: impl Into<String>) -> SwaplockApiError {
    SwaplockApiError::UnexpectedResponse {
        method,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_object_parser_preserves_missing_get_objects_slot() {
        let error = account_object_from_get_objects(json!([null]), "1.2.0")
            .expect_err("missing object fails");

        assert!(matches!(
            error,
            SwaplockApiError::AccountNotFound { account } if account == "1.2.0"
        ));
    }
}
