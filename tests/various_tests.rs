use bls12_381::G1Projective;
use group::{ff::Field, Group, GroupEncoding};
use rand::{
    rngs::OsRng,
    {CryptoRng, Rng},
};

use sigma_rs::toolbox::sigma::{
    transcript::ShakeTranscript, GroupMorphismPreimage, NISigmaProtocol, SchnorrProof,
};

type G = G1Projective;

fn msm_pr<G: Group>(scalars: &[G::Scalar], bases: &[G]) -> G {
    let mut acc = G::identity();
    for (s, p) in scalars.iter().zip(bases.iter()) {
        acc += *p * s;
    }
    acc
}

#[allow(non_snake_case)]
fn discrete_logarithm<G: Group + GroupEncoding>(
    rng: &mut (impl Rng + CryptoRng),
) -> (GroupMorphismPreimage<G>, Vec<G::Scalar>) {
    let mut morphismp: GroupMorphismPreimage<G> = GroupMorphismPreimage::new();

    let var_x: usize = 0;
    let (var_G, var_X): (usize, usize) = (0, 1);
    morphismp.allocate_scalars(1);
    morphismp.allocate_elements(2);
    morphismp.append_equation(var_X, &[(var_x, var_G)]);

    let G = G::generator();
    morphismp.set_elements(&[(var_G, G)]);

    let x = G::Scalar::random(&mut *rng);
    let X = G * x;
    assert!(vec![X] == morphismp.morphism.evaluate(&[x]));

    morphismp.set_elements(&[(var_X, X)]);
    (morphismp, vec![x])
}

#[allow(non_snake_case)]
fn dleq<G: Group + GroupEncoding>(
    rng: &mut (impl Rng + CryptoRng),
) -> (GroupMorphismPreimage<G>, Vec<G::Scalar>) {
    let mut morphismp: GroupMorphismPreimage<G> = GroupMorphismPreimage::new();

    let G = G::generator();
    let H = G::random(&mut *rng);
    let x = G::Scalar::random(&mut *rng);
    let X = G * x;
    let Y = H * x;

    let var_x: usize = 0;
    let (var_G, var_H, var_X, var_Y) = (0, 1, 2, 3);
    morphismp.allocate_scalars(1);
    morphismp.allocate_elements(4);
    morphismp.set_elements(&[(var_G, G), (var_H, H), (var_X, X), (var_Y, Y)]);
    morphismp.append_equation(var_X, &[(var_x, var_G)]);
    morphismp.append_equation(var_Y, &[(var_x, var_H)]);

    assert!(vec![X, Y] == morphismp.morphism.evaluate(&[x]));
    (morphismp, vec![x])
}

#[allow(non_snake_case)]
fn pedersen_commitment<G: Group + GroupEncoding>(
    rng: &mut (impl Rng + CryptoRng),
) -> (GroupMorphismPreimage<G>, Vec<G::Scalar>) {
    let mut morphismp: GroupMorphismPreimage<G> = GroupMorphismPreimage::new();

    let G = G::generator();
    let H = G::random(&mut *rng);
    let x = G::Scalar::random(&mut *rng);
    let r = G::Scalar::random(&mut *rng);
    let witness = vec![x, r];

    let C = G * x + H * r;

    let (var_x, var_r) = (0, 1);
    let (var_G, var_H, var_C) = (0, 1, 2);
    morphismp.allocate_scalars(2);
    morphismp.allocate_elements(3);
    morphismp.set_elements(&[(var_H, H), (var_G, G), (var_C, C)]);
    morphismp.append_equation(var_C, &[(var_x, var_G), (var_r, var_H)]);

    assert!(vec![C] == morphismp.morphism.evaluate(&witness));
    (morphismp, witness)
}

#[allow(non_snake_case)]
fn pedersen_commitment_dleq<G: Group + GroupEncoding>(
    rng: &mut (impl Rng + CryptoRng),
) -> (GroupMorphismPreimage<G>, Vec<G::Scalar>) {
    let mut morphismp: GroupMorphismPreimage<G> = GroupMorphismPreimage::new();

    let mut generators = Vec::<G>::new();
    generators.push(G::random(&mut *rng));
    generators.push(G::random(&mut *rng));
    generators.push(G::random(&mut *rng));
    generators.push(G::random(&mut *rng));

    let mut witness = Vec::<G::Scalar>::new();
    witness.push(G::Scalar::random(&mut *rng));
    witness.push(G::Scalar::random(&mut *rng));

    let X = msm_pr::<G>(&witness, &[generators[0], generators[1]]);
    let Y = msm_pr::<G>(&witness, &[generators[2], generators[3]]);

    let (var_x, var_r) = (0, 1);
    let var_Gs = (0, 1, 2, 3);
    let (var_X, var_Y) = (4, 5);
    morphismp.allocate_scalars(2);
    morphismp.allocate_elements(4);
    morphismp.allocate_elements(2);

    morphismp.set_elements(&[
        (var_Gs.0, generators[0]),
        (var_Gs.1, generators[1]),
        (var_Gs.2, generators[2]),
        (var_Gs.3, generators[3]),
    ]);
    morphismp.set_elements(&[(var_X, X), (var_Y, Y)]);

    morphismp.append_equation(var_X, &[(var_x, var_Gs.0), (var_r, var_Gs.1)]);
    morphismp.append_equation(var_Y, &[(var_x, var_Gs.2), (var_r, var_Gs.3)]);

    assert!(vec![X, Y] == morphismp.morphism.evaluate(&witness));
    (morphismp, witness)
}

#[allow(non_snake_case)]
fn bbs_blind_commitment_computation<G: Group + GroupEncoding>(
    rng: &mut (impl Rng + CryptoRng),
) -> (GroupMorphismPreimage<G>, Vec<G::Scalar>) {
    let mut morphismp: GroupMorphismPreimage<G> = GroupMorphismPreimage::new();

    // length (committed_messages)
    let M = 3;
    // BBS.create_generators(M + 1, "BLIND_" || api_id)
    let (Q_2, J_1, J_2, J_3) = (
        G::random(&mut *rng),
        G::random(&mut *rng),
        G::random(&mut *rng),
        G::random(&mut *rng),
    );
    // BBS.messages_to_scalars(committed_messages,  api_id)
    let (msg_1, msg_2, msg_3) = (
        G::Scalar::random(&mut *rng),
        G::Scalar::random(&mut *rng),
        G::Scalar::random(&mut *rng),
    );

    // these are computed before the proof in the specification
    let secret_prover_blind = G::Scalar::random(&mut *rng);
    let C = Q_2 * secret_prover_blind + J_1 * msg_1 + J_2 * msg_2 + J_3 * msg_3;

    // This is the part that needs to be changed in the specification of blind bbs.
    let (var_secret_prover_blind, var_msg_1, var_msg_2, var_msg_3) = (0, 1, 2, 3);
    let (var_Q_2, var_J_1, var_J_2, var_J_3) = (0, 1, 2, 3);
    let var_C = M + 1;

    morphismp.allocate_scalars(M + 1);
    morphismp.allocate_elements(M + 1);
    morphismp.allocate_elements(1);
    morphismp.set_elements(&[
        (var_Q_2, Q_2),
        (var_J_1, J_1),
        (var_J_2, J_2),
        (var_J_3, J_3),
        (var_C, C),
    ]);

    morphismp.append_equation(
        var_C,
        &[
            (var_secret_prover_blind, var_Q_2),
            (var_msg_1, var_J_1),
            (var_msg_2, var_J_2),
            (var_msg_3, var_J_3),
        ],
    );

    let witness = vec![secret_prover_blind, msg_1, msg_2, msg_3];

    assert!(vec![C] == morphismp.morphism.evaluate(&witness));
    (morphismp, witness)
}

/// This part tests the functioning of morphisms
/// as well as the implementation of GroupMorphismPreimage
#[test]
fn test_discrete_logarithm() {
    let mut rng = OsRng;
    discrete_logarithm::<G>(&mut rng);
}

#[test]
fn test_dleq() {
    let mut rng = OsRng;
    dleq::<G>(&mut rng);
}

#[test]
fn test_pedersen_commitment() {
    let mut rng = OsRng;
    pedersen_commitment::<G>(&mut rng);
}

#[test]
fn test_pedersen_commitment_dleq() {
    let mut rng = OsRng;
    pedersen_commitment_dleq::<G>(&mut rng);
}

#[test]
fn test_bbs_blind_commitment_computation() {
    let mut rng = OsRng;
    bbs_blind_commitment_computation::<G>(&mut rng);
}

/// This part tests the implementation of the SigmaProtocol trait for the
/// SchnorrProof structure as well as the Fiat-Shamir NISigmaProtocol transform
#[allow(non_snake_case)]
#[test]
fn NI_discrete_logarithm() {
    let mut rng = OsRng;
    let (morphismp, witness) = discrete_logarithm::<G>(&mut rng);

    // The SigmaProtocol induced by morphismp
    let protocol = SchnorrProof(morphismp);
    // Fiat-Shamir wrapper
    let domain_sep = b"test-fiat-shamir-schnorr";
    let mut nizk =
        NISigmaProtocol::<SchnorrProof<G>, ShakeTranscript<G>, G>::new(domain_sep, protocol);

    // Prove
    let proof_bytes = nizk.prove(&witness, &mut rng);
    // Verify
    let verified = nizk.verify(&proof_bytes).is_ok();
    assert!(verified, "Fiat-Shamir Schnorr proof verification failed");
}

#[allow(non_snake_case)]
#[test]
fn NI_dleq() {
    let mut rng = OsRng;
    let (morphismp, witness) = dleq::<G>(&mut rng);

    // The SigmaProtocol induced by morphismp
    let protocol = SchnorrProof(morphismp);
    // Fiat-Shamir wrapper
    let domain_sep = b"test-fiat-shamir-DLEQ";
    let mut nizk =
        NISigmaProtocol::<SchnorrProof<G>, ShakeTranscript<G>, G>::new(domain_sep, protocol);

    // Prove
    let proof_bytes = nizk.prove(&witness, &mut rng);
    // Verify
    let verified = nizk.verify(&proof_bytes).is_ok();
    assert!(verified, "DLEQ proof verification failed");
}

#[allow(non_snake_case)]
#[test]
fn NI_pedersen_commitment() {
    let mut rng = OsRng;
    let (morphismp, witness) = pedersen_commitment::<G>(&mut rng);

    // The SigmaProtocol induced by morphismp
    let protocol = SchnorrProof(morphismp);
    // Fiat-Shamir wrapper
    let domain_sep = b"test-fiat-shamir-pedersen-commitment";
    let mut nizk =
        NISigmaProtocol::<SchnorrProof<G>, ShakeTranscript<G>, G>::new(domain_sep, protocol);

    // Prove
    let proof_bytes = nizk.prove(&witness, &mut rng);
    // Verify
    let verified = nizk.verify(&proof_bytes).is_ok();
    assert!(verified, "DLEQ proof verification failed");
}

#[allow(non_snake_case)]
#[test]
fn NI_pedersen_commitment_dleq() {
    let mut rng = OsRng;
    let (morphismp, witness) = pedersen_commitment_dleq::<G>(&mut rng);

    // The SigmaProtocol induced by morphismp
    let protocol = SchnorrProof(morphismp);
    // Fiat-Shamir wrapper
    let domain_sep = b"test-fiat-shamir-pedersen-commitment-DLEQ";
    let mut nizk =
        NISigmaProtocol::<SchnorrProof<G>, ShakeTranscript<G>, G>::new(domain_sep, protocol);

    // Prove
    let proof_bytes = nizk.prove(&witness, &mut rng);
    // Verify
    let verified = nizk.verify(&proof_bytes).is_ok();
    assert!(verified, "DLEQ proof verification failed");
}

#[allow(non_snake_case)]
#[test]
fn NI_bbs_blind_commitment_computation() {
    let mut rng = OsRng;
    let (morphismp, witness) = bbs_blind_commitment_computation::<G>(&mut rng);

    // The SigmaProtocol induced by morphismp
    let protocol = SchnorrProof(morphismp);
    // Fiat-Shamir wrapper
    let domain_sep = b"test-fiat-shamir-bbs-blind-commitment-computation";
    let mut nizk =
        NISigmaProtocol::<SchnorrProof<G>, ShakeTranscript<G>, G>::new(domain_sep, protocol);

    // Prove
    let proof_bytes = nizk.prove(&witness, &mut rng);
    // Verify
    let verified = nizk.verify(&proof_bytes).is_ok();
    assert!(verified, "DLEQ proof verification failed");
}
