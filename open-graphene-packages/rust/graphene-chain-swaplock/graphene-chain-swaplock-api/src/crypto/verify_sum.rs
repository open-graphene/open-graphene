use open_graphene_transport::{GrapheneSession, crypto_api};

use crate::SwaplockApiError;

use super::error::{decode, encode};
use super::types::Commitment;

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
    let result = crypto_api::verify_sum(session, commits, neg_commits, excess)?;
    decode("verify_sum", result)
}
