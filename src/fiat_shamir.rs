//! Fiat-Shamir transformation for Sigma protocols.
//!
//! This module defines `NISigmaProtocol`, a generic non-interactive Sigma protocol wrapper,
//! based on applying the Fiat-Shamir heuristic using a codec.
//!
//! It transforms an interactive Sigma protocol into a non-interactive one,
//! by deriving challenges deterministically from previous protocol messages
//! via a cryptographic sponge function (Codec).
//!
//! # Usage
//! This struct is generic over:
//! - `P`: the underlying Sigma protocol (`SigmaProtocol` trait).
//! - `C`: the codec (`Codec` trait).
//! - `G`: the group used for commitments and operations (`Group` trait).

use crate::{codec::Codec, CompactProtocol, ProofError, SigmaProtocol};

use group::{Group, GroupEncoding};
use rand::{CryptoRng, RngCore};

type Transcript<P> = (
    <P as SigmaProtocol>::Commitment,
    <P as SigmaProtocol>::Challenge,
    <P as SigmaProtocol>::Response,
);

/// A Fiat-Shamir transformation of a Sigma protocol into a non-interactive proof.
///
/// `NISigmaProtocol` wraps an interactive Sigma protocol `P`
/// and a hash-based codec `C`, to produce non-interactive proofs.
///
/// It manages the domain separation, codec reset,
/// proof generation, and proof verification.
///
/// # Type Parameters
/// - `P`: the Sigma protocol implementation.
/// - `C`: the codec used for Fiat-Shamir.
/// - `G`: the group on which the protocol operates.
pub struct NISigmaProtocol<P, C, G>
where
    G: Group + GroupEncoding,
    P: SigmaProtocol<Commitment = Vec<G>, Challenge = <G as Group>::Scalar>,
    C: Codec<Challenge = <G as Group>::Scalar>,
{
    /// Current codec state.
    pub hash_state: C,
    /// Underlying Sigma protocol.
    pub sigmap: P,
}

// QUESTION: Is the morphism supposed to be written to the transcript? I don't see that here.
impl<P, C, G> NISigmaProtocol<P, C, G>
where
    G: Group + GroupEncoding,
    P: SigmaProtocol<Commitment = Vec<G>, Challenge = <G as Group>::Scalar>,
    C: Codec<Challenge = <G as Group>::Scalar> + Clone,
{
    /// Creates a new non-interactive Sigma protocol, identified by a domain separator (usually fixed per protocol instantiation), and an initialized Sigma protocol instance.
    pub fn new(iv: &[u8], instance: P) -> Self {
        let hash_state = C::new(iv);
        Self {
            hash_state,
            sigmap: instance,
        }
    }

    /// Produces a non-interactive proof for a witness.
    pub fn prove(
        &mut self,
        witness: &P::Witness,
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Transcript<P>, ProofError> {
        // QUESTION: Why is the self mutable? It's unclear whether the intention is to have a
        // single NISigmaProtocol be used multiple times, or not. E.g. is the intention that
        // someone might call `proto.verify(commit1, chal1, res1); proto.verify(commit2, chal2, res2)`
        // both operations to contribute to the same transcript? If so, then why is the hash_state
        // cloned here? And if not, why make the receiver mutable? Another option is to have the
        // receiver take ownership of self, if the intention is to _enforce_ non-reuse.
        let mut codec = self.hash_state.clone();

        let (commitment, prover_state) = self.sigmap.prover_commit(witness, rng)?;
        // Commitment data for challenge generation
        let mut data = Vec::new();
        for commit in &commitment {
            data.extend_from_slice(commit.to_bytes().as_ref());
        }
        // Fiat Shamir challenge
        let challenge = codec.prover_message(&data).verifier_challenge();
        // Prover's response
        let response = self.sigmap.prover_response(prover_state, &challenge)?;
        // Local verification of the proof
        self.sigmap.verifier(&commitment, &challenge, &response)?;
        Ok((commitment, challenge, response))
    }

    /// Verify a non-interactive proof and returns a Result: `Ok(())` if the proof verifies successfully, `Err(())` otherwise.
    pub fn verify(
        &mut self,
        commitment: &P::Commitment,
        challenge: &P::Challenge,
        response: &P::Response,
    ) -> Result<(), ProofError> {
        let mut codec = self.hash_state.clone();

        // Commitment data for expected challenge generation
        let mut data = Vec::new();
        for commit in commitment {
            data.extend_from_slice(commit.to_bytes().as_ref());
        }
        // Recompute the challenge
        let expected_challenge = codec.prover_message(&data).verifier_challenge();
        // Verification of the proof
        match *challenge == expected_challenge {
            true => self.sigmap.verifier(commitment, challenge, response),
            false => Err(ProofError::VerificationFailure),
        }
    }

    pub fn prove_batchable(
        &mut self,
        witness: &P::Witness,
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Vec<u8>, ProofError> {
        // NOTE: Returning the commitments as part of a serialized proof might be a barrier in that
        // the commitment is often provided by the verifier, linked to some external message. E.g.
        // it might be a commitment that to a prior state (e.g. balance of a wallet prior to a
        // transaction) for which the prover is showing knowledge of an opening, or it might be
        // calculated as a linear function of other commitments (e.g. subtracting the current
        // timestamp from an issuance timestamp to compute a commitment to the age of a
        // credential).
        let (commitment, challenge, response) = self.prove(witness, rng)?;
        Ok(self
            .sigmap
            .serialize_batchable(&commitment, &challenge, &response)
            .unwrap())
    }

    pub fn verify_batchable(&mut self, proof: &[u8]) -> Result<(), ProofError> {
        let (commitment, response) = self.sigmap.deserialize_batchable(proof).unwrap();

        let mut codec = self.hash_state.clone();

        // Commitment data for expected challenge generation
        let mut data = Vec::new();
        for commit in &commitment {
            data.extend_from_slice(commit.to_bytes().as_ref());
        }
        // Recompute the challenge
        let challenge = codec.prover_message(&data).verifier_challenge();
        // Verification of the proof
        self.sigmap.verifier(&commitment, &challenge, &response)
    }
}

impl<P, C, G> NISigmaProtocol<P, C, G>
where
    G: Group + GroupEncoding,
    P: SigmaProtocol<Commitment = Vec<G>, Challenge = <G as Group>::Scalar> + CompactProtocol,
    C: Codec<Challenge = <G as Group>::Scalar> + Clone,
{
    pub fn prove_compact(
        &mut self,
        witness: &P::Witness,
        rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<Vec<u8>, ProofError> {
        let (commitment, challenge, response) = self.prove(witness, rng)?;
        Ok(self
            .sigmap
            .serialize_compact(&commitment, &challenge, &response)
            .unwrap())
    }

    pub fn verify_compact(&mut self, proof: &[u8]) -> Result<(), ProofError> {
        let (challenge, response) = self.sigmap.deserialize_compact(proof).unwrap();
        // Compute the commitments
        let commitment = self.sigmap.get_commitment(&challenge, &response)?;
        // Verify the proof
        self.verify(&commitment, &challenge, &response)
    }
}
