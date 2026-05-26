use serde_json::{json, Value};

use crate::{GrapheneSession, TransportError};

pub fn get_objects<I, S>(session: &mut GrapheneSession, ids: I) -> Result<Value, TransportError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    session.database_call("get_objects", get_objects_params(ids))
}

fn get_objects_params<I, S>(ids: I) -> Value
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let ids = ids.into_iter().map(Into::into).collect::<Vec<String>>();
    json!([ids])
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
}
