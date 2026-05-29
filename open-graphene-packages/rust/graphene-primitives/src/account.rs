use crate::{ObjectId, ObjectIdError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AccountIdRef(ObjectId);

impl AccountIdRef {
    pub fn parse(value: &str) -> Result<Self, ObjectIdError> {
        Ok(Self(ObjectId::parse(value)?.require_type(1, 2)?))
    }

    pub fn as_object_id(&self) -> &ObjectId {
        &self.0
    }
}

impl std::fmt::Display for AccountIdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_id_ref_validates_account_type() {
        assert_eq!(
            AccountIdRef::parse("1.2.100").unwrap().to_string(),
            "1.2.100"
        );
        assert_eq!(
            AccountIdRef::parse("1.3.0"),
            Err(ObjectIdError::UnexpectedType {
                expected_space: 1,
                expected_type: 2,
                actual_space: 1,
                actual_type: 3,
            })
        );
    }
}
