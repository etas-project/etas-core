use ring::aead::{self, Aad, LessSafeKey, Nonce, UnboundKey};
use zeroize::{Zeroize, Zeroizing};

use super::{ReplayKey, ReplayKeyError, ReplayKeyErrorKind};

impl ReplayKey {
    /// Seal file checkpoint bytes without exporting the provider's secret key.
    pub fn seal_checkpoint(&self, bytes: &mut Vec<u8>) -> Result<[u8; 12], ReplayKeyError> {
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).map_err(|_| {
            ReplayKeyError::new(
                ReplayKeyErrorKind::Unavailable,
                "cannot generate checkpoint nonce",
            )
        })?;
        self.checkpoint_cipher()?
            .seal_in_place_append_tag(
                Nonce::assume_unique_for_key(nonce),
                Aad::from(self.id().to_hex()),
                bytes,
            )
            .map_err(|_| {
                ReplayKeyError::new(ReplayKeyErrorKind::Invalid, "checkpoint encryption failed")
            })?;
        Ok(nonce)
    }

    /// Authenticate before returning any plaintext to the file codec.
    pub fn open_checkpoint(
        &self,
        nonce: [u8; 12],
        bytes: &mut Vec<u8>,
    ) -> Result<(), ReplayKeyError> {
        let cipher = self.checkpoint_cipher()?;
        match cipher.open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::from(self.id().to_hex()),
            bytes,
        ) {
            Ok(plaintext) => {
                let len = plaintext.len();
                bytes.truncate(len);
                Ok(())
            }
            Err(_) => {
                bytes.zeroize();
                bytes.clear();
                Err(ReplayKeyError::new(
                    ReplayKeyErrorKind::Invalid,
                    "checkpoint authentication failed",
                ))
            }
        }
    }

    fn checkpoint_cipher(&self) -> Result<LessSafeKey, ReplayKeyError> {
        // Domain separation keeps file encryption independent of request fingerprints.
        let material = Zeroizing::new(blake3::derive_key(
            "etas.host.checkpoint.encryption-key.v1",
            self.material.as_ref(),
        ));
        UnboundKey::new(&aead::AES_256_GCM, material.as_ref())
            .map(LessSafeKey::new)
            .map_err(|_| {
                ReplayKeyError::new(
                    ReplayKeyErrorKind::Invalid,
                    "invalid checkpoint encryption key",
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_sealing_preserves_bytes_and_uses_fresh_nonces() {
        let key = ReplayKey::generate().unwrap();
        let original = b"fake secret checkpoint payload";
        let mut first = original.to_vec();
        let mut second = original.to_vec();
        let nonce = key.seal_checkpoint(&mut first).unwrap();
        let other = key.seal_checkpoint(&mut second).unwrap();
        assert_ne!(nonce, other);
        assert_ne!(first, second);
        assert!(!first.windows(original.len()).any(|part| part == original));
        key.open_checkpoint(nonce, &mut first).unwrap();
        key.open_checkpoint(other, &mut second).unwrap();
        assert_eq!(first, original);
        assert_eq!(second, original);
    }

    #[test]
    fn checkpoint_sealing_rejects_wrong_keys_nonces_and_damaged_ciphertext() {
        let key = ReplayKey::generate().unwrap();
        let wrong = ReplayKey::generate().unwrap();
        let mut sealed = b"fake secret checkpoint payload".to_vec();
        let nonce = key.seal_checkpoint(&mut sealed).unwrap();
        let mut bytes = sealed.clone();
        assert!(wrong.open_checkpoint(nonce, &mut bytes).is_err());
        assert!(bytes.is_empty());
        let mut other = nonce;
        other[0] ^= 1;
        let mut bytes = sealed.clone();
        assert!(key.open_checkpoint(other, &mut bytes).is_err());
        assert!(bytes.is_empty());
        for index in 0..sealed.len() {
            let mut bytes = sealed.clone();
            bytes[index] ^= 1;
            assert!(key.open_checkpoint(nonce, &mut bytes).is_err());
            assert!(bytes.is_empty());
        }
        for len in [0, 1, sealed.len() - 1] {
            let mut bytes = sealed[..len].to_vec();
            assert!(key.open_checkpoint(nonce, &mut bytes).is_err());
            assert!(bytes.is_empty());
        }
    }
}
