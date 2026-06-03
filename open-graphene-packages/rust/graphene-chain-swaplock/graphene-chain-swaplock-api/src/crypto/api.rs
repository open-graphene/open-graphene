use open_graphene_transport::GrapheneSession;

use super::blind::BlindRequest;
use super::blind_sum::BlindSumRequest;
use super::range_get_info::RangeGetInfoRequest;
use super::range_proof_sign::RangeProofSignRequest;
use super::types::{BlindingFactor, Commitment, RangeProof};
use super::verify_range::VerifyRangeRequest;
use super::verify_range_proof_rewind::VerifyRangeProofRewindRequest;
use super::verify_sum::VerifySumRequest;

/// Entry point for the Swaplock `crypto` API (Pedersen commitments and range
/// proofs for confidential/blinded transfers).
pub struct CryptoApi<'session> {
    pub(crate) session: &'session mut GrapheneSession,
}

impl<'session> CryptoApi<'session> {
    /// Create a Pedersen commitment `commit = blind·G + value·H`.
    pub fn blind(self, blinding_factor: BlindingFactor, value: u64) -> BlindRequest<'session> {
        BlindRequest {
            session: self.session,
            blinding_factor,
            value,
        }
    }

    /// Sum blinding factors: the first `non_neg` are added, the rest subtracted.
    pub fn blind_sum(
        self,
        blinds_in: Vec<BlindingFactor>,
        non_neg: u32,
    ) -> BlindSumRequest<'session> {
        BlindSumRequest {
            session: self.session,
            blinds_in,
            non_neg,
        }
    }

    /// Verify that positive commitments minus negative ones equal `excess`·H.
    pub fn verify_sum(
        self,
        commits_in: Vec<Commitment>,
        neg_commits_in: Vec<Commitment>,
        excess: i64,
    ) -> VerifySumRequest<'session> {
        VerifySumRequest {
            session: self.session,
            commits_in,
            neg_commits_in,
            excess,
        }
    }

    /// Verify a range proof for a commitment and learn the bounds it proves.
    pub fn verify_range(
        self,
        commit: Commitment,
        proof: RangeProof,
    ) -> VerifyRangeRequest<'session> {
        VerifyRangeRequest {
            session: self.session,
            commit,
            proof,
        }
    }

    /// Produce a range proof that a committed value lies in a range.
    #[allow(clippy::too_many_arguments)]
    pub fn range_proof_sign(
        self,
        min_value: u64,
        commit: Commitment,
        commit_blind: BlindingFactor,
        nonce: BlindingFactor,
        base10_exp: i8,
        min_bits: u8,
        actual_value: u64,
    ) -> RangeProofSignRequest<'session> {
        RangeProofSignRequest {
            session: self.session,
            min_value,
            commit,
            commit_blind,
            nonce,
            base10_exp,
            min_bits,
            actual_value,
        }
    }

    /// Verify a range proof and recover the hidden value/blind/message via the nonce.
    pub fn verify_range_proof_rewind(
        self,
        nonce: BlindingFactor,
        commit: Commitment,
        proof: RangeProof,
    ) -> VerifyRangeProofRewindRequest<'session> {
        VerifyRangeProofRewindRequest {
            session: self.session,
            nonce,
            commit,
            proof,
        }
    }

    /// Read a proof's exponent, mantissa and covered range.
    pub fn range_get_info(self, proof: RangeProof) -> RangeGetInfoRequest<'session> {
        RangeGetInfoRequest {
            session: self.session,
            proof,
        }
    }
}
