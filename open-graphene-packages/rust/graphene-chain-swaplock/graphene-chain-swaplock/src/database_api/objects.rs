use std::error::Error;
use std::fmt::Display;

use serde::de::DeserializeOwned;
use serde_json::Value;

pub(super) fn typed_object_from_get_objects_result<T, Id, ParseId, IdOf, ParseError>(
    object_kind: &'static str,
    requested_id: &Id,
    value: Value,
    parse_id: ParseId,
    id_of: IdOf,
) -> Result<Option<T>, Box<dyn Error>>
where
    T: DeserializeOwned,
    Id: Display + PartialEq,
    ParseId: Fn(&str) -> Result<Id, ParseError>,
    IdOf: Fn(&T) -> &str,
    ParseError: Error + 'static,
{
    let values = value
        .as_array()
        .ok_or_else(|| format!("get_objects {object_kind} response must be an array"))?;
    if values.len() != 1 {
        return Err(format!(
            "get_objects {object_kind} response must contain exactly one slot, got {}",
            values.len()
        )
        .into());
    }

    let Some(slot) = values.first() else {
        return Err(format!("get_objects {object_kind} response must contain one slot").into());
    };
    if slot.is_null() {
        return Ok(None);
    }

    let object: T = serde_json::from_value(slot.clone())?;
    let returned_id = parse_id(id_of(&object))?;
    if &returned_id != requested_id {
        return Err(format!(
            "get_objects {object_kind} id mismatch: expected {requested_id}, got {returned_id}"
        )
        .into());
    }

    Ok(Some(object))
}
