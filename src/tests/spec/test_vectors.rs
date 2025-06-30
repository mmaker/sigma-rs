use bls12_381::{G1Projective as G, Scalar};
use core::str;
use hex::FromHex;
use json::JsonValue;
use std::fs;

use crate::codec::{ByteSchnorrCodec, KeccakDuplexSponge};
use crate::fiat_shamir::NISigmaProtocol;
use crate::tests::spec::{
    custom_schnorr_protocol::SchnorrProtocolCustom, random::SRandom, rng::TestDRNG,
};
use crate::tests::test_utils::{
    bbs_blind_commitment_computation, discrete_logarithm, dleq, pedersen_commitment,
    pedersen_commitment_dleq,
};

type Codec = ByteSchnorrCodec<G, KeccakDuplexSponge>;
type SigmaP = SchnorrProtocolCustom<G>;
type NISigmaP = NISigmaProtocol<SigmaP, Codec>;

/// Macro to generate non-interactive sigma protocols test functions
macro_rules! generate_ni_function {
    ($name:ident, $test_fn:ident, $($param:tt),*) => {
        #[allow(non_snake_case)]
        fn $name(seed: &[u8], iv: [u8; 32]) -> (Vec<Scalar>, Vec<u8>) {
            let mut rng = TestDRNG::new(seed);
            let (instance, witness) = $test_fn($(generate_ni_function!(@arg rng, $param)),*);

            let protocol = SchnorrProtocolCustom(instance);
            let nizk = NISigmaP::from_iv(iv, protocol);

            let proof_bytes = nizk.prove_batchable(&witness, &mut rng).unwrap();
            let verified = nizk.verify_batchable(&proof_bytes).is_ok();
            assert!(verified, "Fiat-Shamir Schnorr proof verification failed");
            (witness, proof_bytes)
        }
    };

    (@arg $rng:ident, $type:ident) => {
        G::$type(&mut $rng)
    };
    (@arg $rng:ident, [$type:ident; $count:expr]) => {
        (0..$count).map(|_| G::$type(&mut $rng)).collect::<Vec<_>>().try_into().unwrap()
    };
}

generate_ni_function!(NI_discrete_logarithm, discrete_logarithm, srandom);
generate_ni_function!(NI_dleq, dleq, prandom, srandom);
generate_ni_function!(
    NI_pedersen_commitment,
    pedersen_commitment,
    prandom,
    srandom,
    srandom
);
generate_ni_function!(
    NI_pedersen_commitment_dleq,
    pedersen_commitment_dleq,
    [prandom; 4],
    [srandom; 2]
);
generate_ni_function!(
    NI_bbs_blind_commitment_computation,
    bbs_blind_commitment_computation,
    [prandom; 4],
    [srandom; 3],
    srandom
);

#[allow(clippy::type_complexity)]
#[allow(non_snake_case)]
#[test]
fn test_spec_testvectors() {
    let seed = b"hello world";
    let iv = *b"yellow submarineyellow submarine";

    let vectors = extract_vectors("src/tests/spec/allVectors.json").unwrap();

    let functions: [fn(&[u8], [u8; 32]) -> (Vec<Scalar>, Vec<u8>); 5] = [
        NI_discrete_logarithm,
        NI_dleq,
        NI_pedersen_commitment,
        NI_pedersen_commitment_dleq,
        NI_bbs_blind_commitment_computation,
    ];

    for (i, f) in functions.iter().enumerate() {
        let (_, proof_bytes) = f(seed, iv);
        assert_eq!(
            iv.as_slice(),
            vectors[i].0.as_slice(),
            "context for test vector {i} does not match"
        );
        assert_eq!(
            proof_bytes, vectors[i].1,
            "proof bytes for test vector {i} does not match"
        );
    }
}

fn extract_vectors(path: &str) -> json::Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let content = fs::read_to_string(path).expect("Unable to read JSON file");
    let root: JsonValue = json::parse(&content).expect("JSON parsing error");

    root.entries()
        .map(|(_, obj)| {
            let context_hex = obj["Context"]
                .as_str()
                .expect("Context field not found or not a string");
            let proof_hex = obj["Proof"]
                .as_str()
                .expect("Proof field not found or not a string");
            Ok((
                Vec::from_hex(context_hex).unwrap(),
                Vec::from_hex(proof_hex).unwrap(),
            ))
        })
        .collect()
}
