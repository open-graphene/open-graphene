use serde_json::Value;

pub(super) fn find_object_by_id(value: &Value, id: &str) -> Option<Value> {
    if value
        .as_object()
        .and_then(|object| object.get("id"))
        .and_then(Value::as_str)
        == Some(id)
    {
        return Some(value.clone());
    }

    value
        .as_array()
        .and_then(|items| items.iter().find_map(|item| find_object_by_id(item, id)))
}
