//
// Authors:
// - Nugzari Uzoevi <nougzarm@icloud.com>
// - Michele Orrù <m@orru.net>
// - Lénaïck Gouriou <lg@leanear.io>

#![allow(non_snake_case)]
#![doc(html_logo_url = "https://mmaker.github.io/sigma-rs/")]
//! ## Note
//!

#![deny(unused_variables)]
#![deny(unused_mut)]

pub mod composition;
pub mod errors;
pub mod fiat_shamir;
pub mod serialization;
pub mod linear_relation;
pub mod schnorr_protocol;
pub mod traits;

pub mod codec;
pub mod duplex_sponge;

#[cfg(test)]
pub mod tests;

pub use linear_relation::LinearRelation;
