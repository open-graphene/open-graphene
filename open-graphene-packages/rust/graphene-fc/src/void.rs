//! Graphene's `void_t` — the "no payload" arm of a static variant.

use crate::FcSerialize;
use serde::de::{Deserialize, Deserializer, Error as _};
use serde::ser::{Serialize, SerializeMap, Serializer};

/// The empty payload carried by a static variant's `void_t` arm.
///
/// Exists because Rust's `()` says "nothing" in a way Graphene does not understand. The two
/// wire formats disagree about how nothing is written:
///
/// - FC binary writes zero bytes, which is what `()` does too;
/// - JSON-RPC expects an **empty object**, `[0,{}]`, while `()` serialises to `null`.
///
/// A node handed `[0,null]` fails with `Bad Cast: Invalid cast from type 'null_type' to
/// Object` — so an operation with an unset `void_t` field, such as a data room created with
/// no subject, is rejected outright. The mirror problem applies on the way back: a payload of
/// `{}` will not deserialise into `()`.
///
/// Deserialisation accepts `null` as well as `{}`, so state written by older clients still
/// reads back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VoidT;

impl Serialize for VoidT {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_map(Some(0))?.end()
    }
}

impl<'de> Deserialize<'de> for VoidT {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct VoidVisitor;

        impl<'de> serde::de::Visitor<'de> for VoidVisitor {
            type Value = VoidT;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an empty object, or null")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(VoidT)
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(VoidT)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                // A void payload carries no fields; anything present means the sender and
                // this type disagree about the shape, which is worth surfacing rather than
                // silently dropping.
                if map
                    .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
                    .is_some()
                {
                    return Err(A::Error::custom("void_t payload must be an empty object"));
                }
                Ok(VoidT)
            }
        }

        deserializer.deserialize_any(VoidVisitor)
    }
}

impl FcSerialize for VoidT {
    /// Writes nothing: in FC binary a `void_t` arm is its tag and no payload.
    fn fc_serialize(&self, _out: &mut Vec<u8>) -> crate::Result<()> {
        Ok(())
    }
}

impl utoipa::PartialSchema for VoidT {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::Schema::Object(
            utoipa::openapi::schema::ObjectBuilder::new()
                .description(Some("Graphene void_t: an empty object."))
                .schema_type(utoipa::openapi::schema::SchemaType::Type(
                    utoipa::openapi::schema::Type::Object,
                ))
                .build(),
        )
        .into()
    }
}

impl utoipa::ToSchema for VoidT {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialises_as_empty_object_not_null() {
        // The whole point: `()` would produce `null` here, which nodes reject.
        assert_eq!(serde_json::to_string(&VoidT).unwrap(), "{}");
    }

    #[test]
    fn reads_back_empty_object() {
        assert_eq!(serde_json::from_str::<VoidT>("{}").unwrap(), VoidT);
    }

    #[test]
    fn tolerates_null_from_older_clients() {
        assert_eq!(serde_json::from_str::<VoidT>("null").unwrap(), VoidT);
    }

    #[test]
    fn rejects_a_payload_that_is_not_empty() {
        assert!(serde_json::from_str::<VoidT>(r#"{"a":1}"#).is_err());
    }

    #[test]
    fn writes_no_bytes_in_fc_binary() {
        let mut out = Vec::new();
        VoidT.fc_serialize(&mut out).unwrap();
        assert!(
            out.is_empty(),
            "void_t must not add bytes to a signed transaction"
        );
    }
}
