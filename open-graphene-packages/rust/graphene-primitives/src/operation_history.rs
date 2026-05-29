use crate::{ObjectId, ObjectIdError};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationHistoryIdRef(ObjectId);

impl OperationHistoryIdRef {
    pub fn parse(value: &str) -> Result<Self, ObjectIdError> {
        Ok(Self(ObjectId::parse(value)?.require_type(1, 11)?))
    }

    pub fn as_object_id(&self) -> &ObjectId {
        &self.0
    }
}

impl std::fmt::Display for OperationHistoryIdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_history_id_ref_validates_operation_history_type() {
        assert_eq!(
            OperationHistoryIdRef::parse("1.11.45").unwrap().to_string(),
            "1.11.45"
        );
        assert_eq!(
            OperationHistoryIdRef::parse("1.2.100"),
            Err(ObjectIdError::UnexpectedType {
                expected_space: 1,
                expected_type: 11,
                actual_space: 1,
                actual_type: 2,
            })
        );
    }
}
