use anyhow::{anyhow, Ok, Result};
use num_traits::FromPrimitive;
use once_cell::sync::Lazy;
use ring::rand::{SecureRandom, SystemRandom};
use ring::signature::EcdsaKeyPair;
use rsa::pkcs1::EncodeRsaPrivateKey;
use rsa::rand_core::OsRng;
use rsa::{BigUint, RsaPrivateKey};

use crate::{CryptoNamedCurve, KeyGenAlgorithm, Sha};

static PUB_EXPONENT_1: Lazy<BigUint> = Lazy::new(|| BigUint::from_u64(3).unwrap());
static PUB_EXPONENT_2: Lazy<BigUint> = Lazy::new(|| BigUint::from_u64(65537).unwrap());

pub fn generate_key(algorithm: KeyGenAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        KeyGenAlgorithm::Rsa {
            modulus_length,
            ref public_exponent,
        } => {
            let exponent = BigUint::from_bytes_be(public_exponent);

            if exponent != *PUB_EXPONENT_1 && exponent != *PUB_EXPONENT_2 {
                return Err(anyhow!("Bad public exponent"));
            }

            let mut rng = OsRng;

            let private_key =
                RsaPrivateKey::new_with_exp(&mut rng, modulus_length as usize, &exponent)
                    .map_err(|_| anyhow!("Failed to generate RSA key"))?;

            let private_key = private_key
                .to_pkcs1_der()
                .map_err(|_| anyhow!("Failed to serialize RSA key"))?;

            Ok(private_key.as_bytes().to_vec())
        }
        KeyGenAlgorithm::Ec { curve } => {
            let curve = match curve {
                CryptoNamedCurve::P256 => &ring::signature::ECDSA_P256_SHA256_FIXED_SIGNING,
                CryptoNamedCurve::P384 => &ring::signature::ECDSA_P384_SHA384_FIXED_SIGNING,
            };
            let rng = SystemRandom::new();
            let pkcs8 = EcdsaKeyPair::generate_pkcs8(curve, &rng)
                .map_err(|_| anyhow!("Failed to generate EC key"))?;

            Ok(pkcs8.as_ref().to_vec())
        }
        KeyGenAlgorithm::Aes { length } => {
            let length = length as usize;

            if length % 8 != 0 || length > 256 {
                return Err(anyhow!("Invalid AES key length"));
            }

            let mut key = vec![0u8; length / 8];
            let rng = SystemRandom::new();
            rng.fill(&mut key)
                .map_err(|_| anyhow!("Failed to generate key"))?;

            Ok(key)
        }
        KeyGenAlgorithm::Hmac { hash, length } => {
            let hash = match hash {
                Sha::Sha1 => &ring::hmac::HMAC_SHA1_FOR_LEGACY_USE_ONLY,
                Sha::Sha256 => &ring::hmac::HMAC_SHA256,
                Sha::Sha384 => &ring::hmac::HMAC_SHA384,
                Sha::Sha512 => &ring::hmac::HMAC_SHA512,
            };

            let length = if let Some(length) = length {
                if length % 8 != 0 {
                    return Err(anyhow!("Invalid HMAC key length"));
                }

                let length = length / 8;

                if length > ring::digest::MAX_BLOCK_LEN.try_into().unwrap() {
                    return Err(anyhow!("Invalid HMAC key length"));
                }

                length as usize
            } else {
                hash.digest_algorithm().block_len
            };

            let rng = ring::rand::SystemRandom::new();
            let mut key = vec![0u8; length];
            rng.fill(&mut key)
                .map_err(|_| anyhow!("Failed to generate key"))?;

            Ok(key)
        }
    }
}
