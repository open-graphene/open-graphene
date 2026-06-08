use open_graphene_transport::GrapheneSession;
use serde_json::{Value, json};

use crate::SwaplockApiError;

use super::types::Commitment;
use crate::codec::{decode, encode};

/// Builder for `crypto.verify_sum`: check that the positive commitments minus the
/// negative ones equal `excess`·H. The core check behind balanced confidential transfers.
pub struct VerifySumRequest<'session> {
    pub(super) session: &'session mut GrapheneSession,
    pub(super) commits_in: Vec<Commitment>,
    pub(super) neg_commits_in: Vec<Commitment>,
    pub(super) excess: i64,
}

impl VerifySumRequest<'_> {
    pub async fn get(self) -> Result<bool, SwaplockApiError> {
        verify_sum(
            self.session,
            self.commits_in,
            self.neg_commits_in,
            self.excess,
        )
        .await
    }
}

pub(super) async fn verify_sum(
    session: &mut GrapheneSession,
    commits_in: Vec<Commitment>,
    neg_commits_in: Vec<Commitment>,
    excess: i64,
) -> Result<bool, SwaplockApiError> {
    let commits = encode("verify_sum", &commits_in)?;
    let neg_commits = encode("verify_sum", &neg_commits_in)?;
    let result = session.crypto_call(
        "verify_sum",
        verify_sum_params(commits, neg_commits, excess),
    )?;
    decode("verify_sum", result)
}

fn verify_sum_params(commits_in: Value, neg_commits_in: Value, excess: i64) -> Value {
    json!([commits_in, neg_commits_in, excess])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_sum_params_keep_commits_negatives_and_excess() {
        assert_eq!(
            verify_sum_params(json!(["aa"]), json!(["bb"]), -3),
            json!([["aa"], ["bb"], -3])
        );
    }
}
