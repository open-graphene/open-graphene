use crate::{ObjectId, ObjectIdError};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_id_ref_validates_asset_type() {
        assert_eq!(AssetIdRef::parse("1.3.0").unwrap().to_string(), "1.3.0");
        assert_eq!(
            AssetIdRef::parse("1.2.100"),
            Err(ObjectIdError::UnexpectedType {
                expected_space: 1,
                expected_type: 3,
                actual_space: 1,
                actual_type: 2,
            })
        );
    }
}
