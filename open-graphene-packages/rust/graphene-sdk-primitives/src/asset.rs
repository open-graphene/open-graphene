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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetAmount {
    pub amount: i64,
    pub asset_id: AssetIdRef,
}

impl AssetAmount {
    pub fn new(amount: i64, asset_id: AssetIdRef) -> Self {
        Self { amount, asset_id }
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

    #[test]
    fn asset_amount_wraps_raw_amount_and_asset_ref() {
        let asset_id = AssetIdRef::parse("1.3.0").unwrap();
        let amount = AssetAmount::new(42, asset_id.clone());

        assert_eq!(amount.amount, 42);
        assert_eq!(amount.asset_id, asset_id);
    }
}
