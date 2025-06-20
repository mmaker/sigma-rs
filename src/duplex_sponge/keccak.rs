//! Keccak-based duplex sponge implementation
//!
//! This module implements a duplex sponge construction using the Keccak-f[1600] permutation.
//! It is designed to match test vectors from the original Sage implementation.

use crate::duplex_sponge::DuplexSpongeInterface;
use zerocopy::IntoBytes;

const RATE: usize = 136;
const LENGTH: usize = 136 + 64;

/// Low-level Keccak-f[1600] state representation.
#[derive(Clone, Default)]
pub struct KeccakPermutationState([u64; LENGTH / 8]);

impl KeccakPermutationState {
    pub fn new(iv: [u8; 32]) -> Self {
        let mut state = Self::default();
        state.as_mut()[RATE..RATE + 32].copy_from_slice(&iv);
        state
    }

    pub fn permute(&mut self) {
        keccak::f1600(&mut self.0);
    }
}

impl AsRef<[u8]> for KeccakPermutationState {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl AsMut<[u8]> for KeccakPermutationState {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut_bytes()
    }
}

/// Duplex sponge construction using Keccak-f[1600].
#[derive(Clone)]
pub struct KeccakDuplexSponge {
    state: KeccakPermutationState,
    absorb_index: usize,
    squeeze_index: usize,
}

impl KeccakDuplexSponge {
    pub fn new(iv: [u8; 32]) -> Self {
        let state = KeccakPermutationState::new(iv);
        KeccakDuplexSponge {
            state,
            absorb_index: 0,
            squeeze_index: RATE,
        }
    }
}

impl DuplexSpongeInterface for KeccakDuplexSponge {
    fn new(iv: [u8; 32]) -> Self {
        KeccakDuplexSponge::new(iv)
    }

    fn absorb(&mut self, mut input: &[u8]) {
        self.squeeze_index = RATE;

        while !input.is_empty() {
            if self.absorb_index == RATE {
                self.state.permute();
                self.absorb_index = 0;
            }

            let chunk_size = usize::min(RATE - self.absorb_index, input.len());
            let dest = &mut self.state.as_mut()[self.absorb_index..self.absorb_index + chunk_size];
            dest.copy_from_slice(&input[..chunk_size]);
            self.absorb_index += chunk_size;
            input = &input[chunk_size..];
        }
    }

    fn squeeze(&mut self, mut length: usize) -> Vec<u8> {
        if length == 0 {
            return Vec::new();
        }
        self.absorb_index = RATE;

        let mut output = Vec::new();
        while length != 0 {
            if self.squeeze_index == RATE {
                self.state.permute();
                self.squeeze_index = 0;
            }

            let chunk_size = usize::min(RATE - self.squeeze_index, length);
            output.extend_from_slice(
                &self.state.as_mut()[self.squeeze_index..self.squeeze_index + chunk_size],
            );
            self.squeeze_index += chunk_size;
            length -= chunk_size;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::duplex_sponge::DuplexSpongeInterface;

    #[test]
    fn test_keccak_duplex_sponge() {
        let mut sponge = KeccakDuplexSponge::new(*b"unit_tests_keccak_tag___________");

        let input = b"Hello, World!";
        sponge.absorb(input);
        let output = sponge.squeeze(64);

        assert_eq!(output, hex::decode("73e4a040a956f57693fb2b2dde8a8ea2c14d39ff8830060cd0301d6de25b2097ba858efedeeb89368eaf7c94a68f62835f932b5f0dd0ba376c48a0fdb5e21f0c").unwrap());
    }

    #[test]
    fn test_absorb_empty_before_does_not_break() {
        let mut sponge = KeccakDuplexSponge::new(*b"unit_tests_keccak_tag___________");
        sponge.absorb(b"");
        sponge.absorb(b"Hello, World!");
        sponge.squeeze(0);
        let output = sponge.squeeze(64);

        assert_eq!(output, hex::decode("73e4a040a956f57693fb2b2dde8a8ea2c14d39ff8830060cd0301d6de25b2097ba858efedeeb89368eaf7c94a68f62835f932b5f0dd0ba376c48a0fdb5e21f0c").unwrap());
    }
    #[test]
    fn test_absorb_empty_after_does_not_break() {
        let mut sponge = KeccakDuplexSponge::new(*b"unit_tests_keccak_tag___________");
        sponge.absorb(b"Hello, World!");
        sponge.absorb(b"");
        sponge.squeeze(0);
        let output = sponge.squeeze(64);

        assert_eq!(output, hex::decode("73e4a040a956f57693fb2b2dde8a8ea2c14d39ff8830060cd0301d6de25b2097ba858efedeeb89368eaf7c94a68f62835f932b5f0dd0ba376c48a0fdb5e21f0c").unwrap());
    }

    #[test]
    fn test_squeeze_zero_behavior() {
        let mut sponge = KeccakDuplexSponge::new(*b"unit_tests_keccak_tag___________");
        sponge.squeeze(0);
        sponge.absorb(b"Hello, World!");
        sponge.squeeze(0);
        let output = sponge.squeeze(64);

        assert_eq!(output, hex::decode("73e4a040a956f57693fb2b2dde8a8ea2c14d39ff8830060cd0301d6de25b2097ba858efedeeb89368eaf7c94a68f62835f932b5f0dd0ba376c48a0fdb5e21f0c").unwrap());
    }

    #[test]
    fn test_absorb_squeeze_absorb_consistency() {
        let tag = *b"edge-case-test-domain-absorb0000";

        let mut sponge = KeccakDuplexSponge::new(tag);
        sponge.absorb(b"first");
        sponge.squeeze(32);
        sponge.absorb(b"second");
        let output = sponge.squeeze(32);

        assert_eq!(
            output,
            hex::decode("5b89db635853345429206e79f6ba536b83a429b4070443512c498419834cb78e")
                .unwrap()
        );
    }
    #[test]
    fn test_associativity_of_absorb() {
        let expected_output =
            hex::decode("7dfada182d6191e106ce287c2262a443ce2fb695c7cc5037a46626e88889af58")
                .unwrap();
        let tag = *b"absorb-associativity-domain-----";

        // Absorb all at once
        let mut sponge1 = KeccakDuplexSponge::new(tag);
        sponge1.absorb(b"hello world");
        let out1 = sponge1.squeeze(32);

        // Absorb in two parts
        let mut sponge2 = KeccakDuplexSponge::new(tag);
        sponge2.absorb(b"hello");
        sponge2.absorb(b" world");
        let out2 = sponge2.squeeze(32);

        assert_eq!(out1.to_vec(), expected_output);
        assert_eq!(out2.to_vec(), expected_output);
    }

    #[test]
    fn test_tag_affects_output() {
        let tag1 = *b"domain-one-differs-here-00000000";
        let tag2 = *b"domain-two-differs-here-00000000";

        let mut sponge1 = KeccakDuplexSponge::new(tag1);
        let mut sponge2 = KeccakDuplexSponge::new(tag2);

        sponge1.absorb(b"input");
        sponge2.absorb(b"input");

        let out1 = sponge1.squeeze(32);
        let out2 = sponge2.squeeze(32);

        assert_eq!(
            out1.to_vec(),
            hex::decode("2ecad63584ec0ff7f31edb822530762e5cb4b7dc1a62b1ffe02c43f3073a61b8")
                .unwrap()
        );
        assert_eq!(
            out2.to_vec(),
            hex::decode("6310fa0356e1bab0442fa19958e1c4a6d1dcc565b2b139b6044d1a809f531825")
                .unwrap()
        );
    }
}
