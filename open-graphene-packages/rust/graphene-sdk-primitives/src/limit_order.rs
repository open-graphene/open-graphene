use crate::{ObjectId, ObjectIdError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LimitOrderIdRef(ObjectId);

impl LimitOrderIdRef {
    pub fn parse(value: &str) -> Result<Self, ObjectIdError> {
        Ok(Self(ObjectId::parse(value)?.require_type(1, 7)?))
    }

    pub fn as_object_id(&self) -> &ObjectId {
        &self.0
    }
}

impl std::fmt::Display for LimitOrderIdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_order_id_ref_validates_limit_order_type() {
        assert_eq!(
            LimitOrderIdRef::parse("1.7.123").unwrap().to_string(),
            "1.7.123"
        );
        assert_eq!(
            LimitOrderIdRef::parse("1.2.100"),
            Err(ObjectIdError::UnexpectedType {
                expected_space: 1,
                expected_type: 7,
                actual_space: 1,
                actual_type: 2,
            })
        );
    }

    #[test]
    fn limit_order_id_ref_rejects_malformed_ids() {
        assert!(matches!(
            LimitOrderIdRef::parse("not-an-id"),
            Err(ObjectIdError::InvalidFormat)
        ));
    }
}
