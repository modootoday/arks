//! Pure-Rust NaCl/libsodium SealedBox (`crypto_box_seal`) for encrypting
//! GitHub Actions secrets client-side against a repository/org/environment
//! X25519 public key (zero C libsodium FFI).
//!
//! Primary references:
//! - GitHub REST API Actions Secrets: https://docs.github.com/en/rest/actions/secrets
//! - Libsodium Sealed Boxes: X25519 ephemeral keypair + BLAKE2b-192(pk_e || pk_r) nonce
//!   + XSalsa20Poly1305 AEAD (`pk_e || ciphertext || mac`, 48-byte overhead).

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;
use crypto_box::PublicKey;
use crypto_box::aead::OsRng;

/// Seal `plaintext` for the Base64-encoded 32-byte X25519 `public_key_b64`,
/// returning the Base64-encoded libsodium sealed box wire payload.
pub fn seal_secret_base64(public_key_b64: &str, plaintext: &[u8]) -> Result<String, String> {
    let raw_pk = B64
        .decode(public_key_b64.trim())
        .map_err(|e| format!("invalid base64 public key: {e}"))?;
    let pk_bytes: [u8; 32] = raw_pk.try_into().map_err(|v: Vec<u8>| {
        format!(
            "invalid X25519 public key length: expected 32 bytes, got {}",
            v.len()
        )
    })?;
    let pk = PublicKey::from_bytes(pk_bytes);
    let sealed = pk
        .seal(&mut OsRng, plaintext)
        .map_err(|e| format!("sealed-box encryption failed: {e}"))?;
    Ok(B64.encode(sealed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crypto_box::SecretKey;

    #[test]
    fn sealed_box_roundtrip_and_overhead() {
        let sk = SecretKey::generate(&mut OsRng);
        let pk_b64 = B64.encode(sk.public_key().as_bytes());
        let secret = b"super-secret-token-value-12345";

        let sealed_b64_1 = seal_secret_base64(&pk_b64, secret).expect("seal 1");
        let sealed_b64_2 = seal_secret_base64(&pk_b64, secret).expect("seal 2");
        // Fresh ephemeral key per call -> distinct ciphertexts.
        assert_ne!(sealed_b64_1, sealed_b64_2);

        let raw = B64.decode(&sealed_b64_1).unwrap();
        // CRYPTO_BOX_SEALBYTES = 32 (ephemeral pk) + 16 (Poly1305 MAC) = 48.
        assert_eq!(raw.len(), secret.len() + 48);

        let opened = sk.unseal(&raw).expect("recipient must unseal");
        assert_eq!(opened, secret);
    }

    #[test]
    fn rejects_invalid_public_keys_fail_closed() {
        assert!(seal_secret_base64("not!!base64", b"x").is_err());
        assert!(seal_secret_base64(&B64.encode([0u8; 16]), b"x").is_err());
    }
}
