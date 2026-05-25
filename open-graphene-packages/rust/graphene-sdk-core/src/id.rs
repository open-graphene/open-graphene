use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId {
    pub space: u8,
    pub type_id: u8,
    pub instance: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ObjectIdError {
    #[error("object id must have form space.type.instance")]
    InvalidFormat,
    #[error("object id component is out of range")]
    ComponentOutOfRange,
    #[error(
        "expected object id space {expected_space}, type {expected_type}, got {actual_space}.{actual_type}"
    )]
    UnexpectedType {
        expected_space: u8,
        expected_type: u8,
        actual_space: u8,
        actual_type: u8,
    },
}

impl ObjectId {
    pub fn parse(value: &str) -> Result<Self, ObjectIdError> {
        let parts = value.split('.').collect::<Vec<_>>();
        if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
            return Err(ObjectIdError::InvalidFormat);
        }
        let space = parts[0]
            .parse::<u8>()
            .map_err(|_| ObjectIdError::ComponentOutOfRange)?;
        let type_id = parts[1]
            .parse::<u8>()
            .map_err(|_| ObjectIdError::ComponentOutOfRange)?;
        let instance = parts[2]
            .parse::<u64>()
            .map_err(|_| ObjectIdError::ComponentOutOfRange)?;
        Ok(Self {
            space,
            type_id,
            instance,
        })
    }

    pub fn require_type(
        self,
        expected_space: u8,
        expected_type: u8,
    ) -> Result<Self, ObjectIdError> {
        if self.space != expected_space || self.type_id != expected_type {
            return Err(ObjectIdError::UnexpectedType {
                expected_space,
                expected_type,
                actual_space: self.space,
                actual_type: self.type_id,
            });
        }
        Ok(self)
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.space, self.type_id, self.instance)
    }
}

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetIdRef(ObjectId);

impl AssetIdRef {
    pub fn parse(value: &str) -> Result<Self, ObjectIdError> {
        Ok(Self(ObjectId::parse(value)?.require_type(1, 3)?))
    }

    pub fn as_object_id(&self) -> &ObjectId {
        &self.0
    }
}

impl std::fmt::Display for AssetIdRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

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
    fn object_id_parses_components() {
        let id = ObjectId::parse("1.2.100").unwrap();
        assert_eq!(id.space, 1);
        assert_eq!(id.type_id, 2);
        assert_eq!(id.instance, 100);
        assert_eq!(id.to_string(), "1.2.100");
    }

    #[test]
    fn object_id_rejects_invalid_shapes() {
        assert_eq!(ObjectId::parse("1.2"), Err(ObjectIdError::InvalidFormat));
        assert_eq!(ObjectId::parse("1.2."), Err(ObjectIdError::InvalidFormat));
        assert_eq!(
            ObjectId::parse("1.x.3"),
            Err(ObjectIdError::ComponentOutOfRange)
        );
    }

    #[test]
    fn typed_refs_validate_expected_type() {
        assert_eq!(
            AccountIdRef::parse("1.2.100").unwrap().to_string(),
            "1.2.100"
        );
        assert_eq!(AssetIdRef::parse("1.3.0").unwrap().to_string(), "1.3.0");
        assert_eq!(
            OperationHistoryIdRef::parse("1.11.45").unwrap().to_string(),
            "1.11.45"
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
