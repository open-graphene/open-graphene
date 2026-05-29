use crate::AssetAmount;
use open_graphene_primitives::AssetIdRef;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BalanceCheck {
    pub balance: AssetAmount,
    pub transfer_amount: AssetAmount,
    pub fee: AssetAmount,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BalanceError {
    #[error("amount plus fee overflows i64")]
    RequiredAmountOverflow,
    #[error("insufficient balance for asset {asset_id}: balance {balance}, required {required}")]
    InsufficientBalance {
        asset_id: AssetIdRef,
        balance: i64,
        required: i64,
    },
}

pub fn ensure_sufficient_balance(check: &BalanceCheck) -> Result<(), BalanceError> {
    let required = if check.fee.asset_id == check.transfer_amount.asset_id {
        check
            .transfer_amount
            .amount
            .checked_add(check.fee.amount)
            .ok_or(BalanceError::RequiredAmountOverflow)?
    } else {
        check.transfer_amount.amount
    };

    if check.balance.amount < required {
        return Err(BalanceError::InsufficientBalance {
            asset_id: check.transfer_amount.asset_id.clone(),
            balance: check.balance.amount,
            required,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(amount: i64, asset_id: &str) -> AssetAmount {
        AssetAmount {
            amount,
            asset_id: AssetIdRef::parse(asset_id).unwrap(),
        }
    }

    #[test]
    fn balance_guard_requires_amount_plus_fee_when_fee_uses_transfer_asset() {
        ensure_sufficient_balance(&BalanceCheck {
            balance: amount(110, "1.3.0"),
            transfer_amount: amount(100, "1.3.0"),
            fee: amount(10, "1.3.0"),
        })
        .unwrap();

        assert_eq!(
            ensure_sufficient_balance(&BalanceCheck {
                balance: amount(109, "1.3.0"),
                transfer_amount: amount(100, "1.3.0"),
                fee: amount(10, "1.3.0"),
            }),
            Err(BalanceError::InsufficientBalance {
                asset_id: AssetIdRef::parse("1.3.0").unwrap(),
                balance: 109,
                required: 110,
            })
        );
    }

    #[test]
    fn balance_guard_requires_only_amount_when_fee_uses_another_asset() {
        ensure_sufficient_balance(&BalanceCheck {
            balance: amount(100, "1.3.0"),
            transfer_amount: amount(100, "1.3.0"),
            fee: amount(10, "1.3.1"),
        })
        .unwrap();

        assert_eq!(
            ensure_sufficient_balance(&BalanceCheck {
                balance: amount(99, "1.3.0"),
                transfer_amount: amount(100, "1.3.0"),
                fee: amount(10, "1.3.1"),
            }),
            Err(BalanceError::InsufficientBalance {
                asset_id: AssetIdRef::parse("1.3.0").unwrap(),
                balance: 99,
                required: 100,
            })
        );
    }

    #[test]
    fn balance_guard_rejects_required_amount_overflow() {
        assert_eq!(
            ensure_sufficient_balance(&BalanceCheck {
                balance: amount(i64::MAX, "1.3.0"),
                transfer_amount: amount(i64::MAX, "1.3.0"),
                fee: amount(1, "1.3.0"),
            }),
            Err(BalanceError::RequiredAmountOverflow)
        );
    }
}
