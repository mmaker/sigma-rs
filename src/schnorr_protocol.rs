//! Implementation of the generic Schnorr Sigma Protocol over a [`Group`].
//!
//! This module defines the [`SchnorrProof`] structure, which implements
//! a Sigma protocol proving different types of discrete logarithm relations (eg. Schnorr, Pedersen's commitments)
//! through a group morphism abstraction (see [Maurer09](https://crypto-test.ethz.ch/publications/files/Maurer09.pdf)).

use crate::errors::Error;
use crate::linear_relation::LinearRelation;
use crate::{
    serialization::{
        deserialize_elements, deserialize_scalars, serialize_elements, serialize_scalars,
    },
    traits::{SigmaProtocol, SigmaProtocolSimulator},
};

use ff::Field;
use group::{Group, GroupEncoding};
use rand::{CryptoRng, Rng, RngCore};

/// A Schnorr protocol proving knowledge of a witness for a linear group relation.
///
/// This implementation generalizes Schnorr’s discrete logarithm proof by using
/// a [`LinearRelation`], representing an abstract linear relation over the group.
///
/// # Type Parameters
/// - `G`: A cryptographic group implementing [`Group`] and [`GroupEncoding`].
#[derive(Clone, Default, Debug)]
pub struct SchnorrProof<G: Group + GroupEncoding>(pub LinearRelation<G>);

impl<G: Group + GroupEncoding> SchnorrProof<G> {
    pub fn witness_length(&self) -> usize {
        self.0.linear_map.num_scalars
    }

    pub fn commitment_length(&self) -> usize {
        self.0.linear_map.num_constraints()
    }
}

impl<G> From<LinearRelation<G>> for SchnorrProof<G>
where
    G: Group + GroupEncoding,
{
    fn from(value: LinearRelation<G>) -> Self {
        Self(value)
    }
}

impl<G> SigmaProtocol for SchnorrProof<G>
where
    G: Group + GroupEncoding,
{
    type Commitment = Vec<G>;
    type ProverState = (Vec<<G as Group>::Scalar>, Vec<<G as Group>::Scalar>);
    type Response = Vec<<G as Group>::Scalar>;
    type Witness = Vec<<G as Group>::Scalar>;
    type Challenge = <G as Group>::Scalar;

    /// Prover's first message: generates a commitment using random nonces.
    ///
    /// # Parameters
    /// - `witness`: A vector of scalars that satisfy the linear map relation.
    /// - `rng`: A cryptographically secure random number generator.
    ///
    /// # Returns
    /// - A tuple containing:
    ///     - The commitment (a vector of group elements).
    ///     - The prover state (random nonces and witness) used to compute the response.
    ///
    /// # Errors
    /// -[`Error::InvalidInstanceWitnessPair`] if the witness vector length is incorrect.
    fn prover_commit(
        &self,
        witness: &Self::Witness,
        mut rng: &mut (impl RngCore + CryptoRng),
    ) -> Result<(Self::Commitment, Self::ProverState), Error> {
        if witness.len() != self.witness_length() {
            return Err(Error::InvalidInstanceWitnessPair);
        }

        // If the relation being proven is trivial, refuse to prove the statement.
        if self.0.image()?.iter().all(|&x| x == G::identity()) {
            return Err(Error::InvalidInstanceWitnessPair)
        }

        let nonces: Vec<G::Scalar> = (0..self.witness_length())
            .map(|_| G::Scalar::random(&mut rng))
            .collect();
        let commitment = self.0.linear_map.evaluate(&nonces)?;
        let prover_state = (nonces, witness.clone());
        Ok((commitment, prover_state))
    }

    /// Computes the prover's response (second message) using the challenge.
    ///
    /// # Parameters
    /// - `state`: The prover state returned by `prover_commit`, typically containing randomness and witness components.
    /// - `challenge`: The verifier's challenge scalar.
    ///
    /// # Returns
    /// - A vector of scalars forming the prover's response.
    ///
    /// # Errors
    /// - Returns [`Error::InvalidInstanceWitnessPair`] if the prover state vectors have incorrect lengths.
    fn prover_response(
        &self,
        prover_state: Self::ProverState,
        challenge: &Self::Challenge,
    ) -> Result<Self::Response, Error> {
        let (nonces, witness) = prover_state;

        if nonces.len() != self.witness_length() || witness.len() != self.witness_length() {
            return Err(Error::InvalidInstanceWitnessPair);
        }

        let responses = nonces
            .into_iter()
            .zip(witness)
            .map(|(r, w)| r + w * challenge)
            .collect();
        Ok(responses)
    }
    /// Verifies the correctness of the proof.
    ///
    /// # Parameters
    /// - `commitment`: The prover's commitment vector (group elements).
    /// - `challenge`: The challenge scalar.
    /// - `response`: The prover's response vector.
    ///
    /// # Returns
    /// - `Ok(())` if the proof is valid.
    /// - `Err(Error::VerificationFailure)` if the proof is invalid.
    /// - `Err(Error::InvalidInstanceWitnessPair)` if the lengths of commitment or response do not match the expected counts.
    ///
    /// # Errors
    /// -[`Error::VerificationFailure`] if the computed relation
    /// does not hold for the provided challenge and response, indicating proof invalidity.
    /// -[`Error::InvalidInstanceWitnessPair`] if the commitment or response length is incorrect.
    fn verifier(
        &self,
        commitment: &Self::Commitment,
        challenge: &Self::Challenge,
        response: &Self::Response,
    ) -> Result<(), Error> {
        if commitment.len() != self.commitment_length() || response.len() != self.witness_length() {
            return Err(Error::InvalidInstanceWitnessPair);
        }

        let lhs = self.0.linear_map.evaluate(response)?;
        let mut rhs = Vec::new();
        for (i, g) in commitment.iter().enumerate() {
            rhs.push({
                let image_var = self.0.image[i];
                self.0.linear_map.group_elements.get(image_var)? * challenge + g
            });
        }
        if lhs == rhs {
            Ok(())
        } else {
            Err(Error::VerificationFailure)
        }
    }

    /// Serializes the prover's commitment into a byte vector.
    ///
    /// This function encodes the vector of group elements (the commitment)
    /// into a binary format suitable for transmission or storage. This is
    /// typically the first message sent in a Sigma protocol round.
    ///
    /// # Parameters
    /// - `commitment`: A vector of group elements representing the prover's commitment.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized group elements.
    fn serialize_commitment(&self, commitment: &Self::Commitment) -> Vec<u8> {
        serialize_elements(commitment)
    }

    /// Serializes the verifier's challenge scalar into bytes.
    ///
    /// Converts the challenge scalar into a fixed-length byte encoding. This can be used
    /// for Fiat–Shamir hashing, transcript recording, or proof transmission.
    ///
    /// # Parameters
    /// - `challenge`: The scalar challenge value.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized scalar.
    fn serialize_challenge(&self, challenge: &Self::Challenge) -> Vec<u8> {
        serialize_scalars::<G>(&[*challenge])
    }

    /// Serializes the prover's response vector into a byte format.
    ///
    /// The response is a vector of scalars computed by the prover after receiving
    /// the verifier's challenge. This function encodes the vector into a format
    /// suitable for transmission or inclusion in a batchable proof.
    ///
    /// # Parameters
    /// - `response`: A vector of scalar responses computed by the prover.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized scalars.
    fn serialize_response(&self, response: &Self::Response) -> Vec<u8> {
        serialize_scalars::<G>(response)
    }

    /// Deserializes a byte slice into a vector of group elements (commitment).
    ///
    /// This function reconstructs the prover’s commitment from its binary representation.
    /// The number of elements expected is determined by the number of linear constraints
    /// in the underlying linear relation.
    ///
    /// # Parameters
    /// - `data`: A byte slice containing the serialized commitment.
    ///
    /// # Returns
    /// A `Vec<G>` containing the deserialized group elements.
    ///
    /// # Errors
    /// - Returns [`Error::VerificationFailure`] if the data is malformed or contains an invalid encoding.
    fn deserialize_commitment(&self, data: &[u8]) -> Result<Self::Commitment, Error> {
        deserialize_elements::<G>(data, self.commitment_length()).ok_or(Error::VerificationFailure)
    }

    /// Deserializes a byte slice into a challenge scalar.
    ///
    /// This function expects a single scalar to be encoded and returns it as the verifier's challenge.
    ///
    /// # Parameters
    /// - `data`: A byte slice containing the serialized scalar challenge.
    ///
    /// # Returns
    /// The deserialized scalar challenge value.
    ///
    /// # Errors
    /// - Returns [`Error::VerificationFailure`] if deserialization fails or data is invalid.
    fn deserialize_challenge(&self, data: &[u8]) -> Result<Self::Challenge, Error> {
        let scalars = deserialize_scalars::<G>(data, 1).ok_or(Error::VerificationFailure)?;
        Ok(scalars[0])
    }

    /// Deserializes a byte slice into the prover's response vector.
    ///
    /// The response vector contains scalars used in the second round of the Sigma protocol.
    /// The expected number of scalars matches the number of witness variables.
    ///
    /// # Parameters
    /// - `data`: A byte slice containing the serialized response.
    ///
    /// # Returns
    /// A vector of deserialized scalars.
    ///
    /// # Errors
    /// - Returns [`Error::VerificationFailure`] if the byte data is malformed or the length is incorrect.
    fn deserialize_response(&self, data: &[u8]) -> Result<Self::Response, Error> {
        deserialize_scalars::<G>(data, self.witness_length()).ok_or(Error::VerificationFailure)
    }

    fn instance_label(&self) -> impl AsRef<[u8]> {
        self.0.label()
    }

    fn protocol_identifier(&self) -> impl AsRef<[u8]> {
        b"SchnorrProof"
    }
}

impl<G> SigmaProtocolSimulator for SchnorrProof<G>
where
    G: Group + GroupEncoding,
{
    /// Simulates a valid transcript for a given challenge without a witness.
    ///
    /// # Parameters
    /// - `challenge`: A scalar value representing the challenge.
    /// - `rng`: A cryptographically secure RNG.
    ///
    /// # Returns
    /// - A commitment and response forming a valid proof for the given challenge.
    fn simulate_response<R: Rng + CryptoRng>(&self, mut rng: &mut R) -> Self::Response {
        let response: Vec<G::Scalar> = (0..self.witness_length())
            .map(|_| G::Scalar::random(&mut rng))
            .collect();
        response
    }

    /// Simulates a full proof transcript using a randomly generated challenge.
    ///
    /// # Parameters
    /// - `rng`: A cryptographically secure RNG.
    ///
    /// # Returns
    /// - A tuple `(commitment, challenge, response)` forming a valid proof.
    fn simulate_transcript<R: Rng + CryptoRng>(
        &self,
        rng: &mut R,
    ) -> Result<(Self::Commitment, Self::Challenge, Self::Response), Error> {
        let challenge = G::Scalar::random(&mut *rng);
        let response = self.simulate_response(&mut *rng);
        let commitment = self.simulate_commitment(&challenge, &response)?;
        Ok((commitment, challenge, response))
    }

    /// Recomputes the commitment from the challenge and response (used in compact proofs).
    ///
    /// # Parameters
    /// - `challenge`: The challenge scalar issued by the verifier or derived via Fiat–Shamir.
    /// - `response`: The prover's response vector.
    ///
    /// # Returns
    /// - A vector of group elements representing the simulated commitment (one per linear constraint).
    ///
    /// # Errors
    /// - [`Error::InvalidInstanceWitnessPair`] if the response length does not match the expected number of scalars.
    fn simulate_commitment(
        &self,
        challenge: &Self::Challenge,
        response: &Self::Response,
    ) -> Result<Self::Commitment, Error> {
        if response.len() != self.witness_length() {
            return Err(Error::InvalidInstanceWitnessPair);
        }

        let response_image = self.0.linear_map.evaluate(response)?;
        let image = self.0.image()?;

        let commitment = response_image
            .iter()
            .zip(&image)
            .map(|(res, img)| *res - *img * challenge)
            .collect::<Vec<_>>();
        Ok(commitment)
    }
}
