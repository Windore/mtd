//! A Module defining networking functions for MTD such as syncing with a remote server or running a
//! server. Data transmitted over the network is encrypted.

use std::fmt::{Display, Formatter};

/// An networking and crypt related error.
#[derive(Debug)]
pub enum Error {
    /// Indicates that encrypting data failed.
    EncryptingErr,
    /// Indicates that decrypting data failed. The two common reasons for this error are incorrect
    /// passwords or tampered ciphertexts.
    DecryptingErr,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::EncryptingErr => {
                write!(f, "Encrypting data failed.")
            }
            Error::DecryptingErr => {
                write!(f, "Decrypting data failed.")
            }
        }
    }
}

/// Module containing functionality for encrypting/decrypting messages used for secure network
/// communication. Data is encrypted with AES-GCM. The encryption key is generated from a password
/// using Argon2. For network communications, session ids should be used in addition to encrypting
/// data.
pub mod crypt {
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use aes_gcm::aead::{Aead, NewAead};
    use argon2::Argon2;
    use rand::random;

    use crate::network::Error;

    /// Encrypts a given byte array with the given password.
    pub fn encrypt(msg: &[u8], passwd: &[u8]) -> Result<Vec<u8>, Error> {
        let key_salt: [u8; 16] = random();
        let argon2 = Argon2::default();

        let mut secret_passwd_hash: [u8; 32] = [0; 32];
        argon2.hash_password_into(passwd, &key_salt, &mut secret_passwd_hash).map_err(|_| Error::EncryptingErr)?;
        let encryption_key = Key::from_slice(&secret_passwd_hash);

        let cipher = Aes256Gcm::new(encryption_key);

        // Random 96-bits for nonce.
        let nonce_bits: [u8; 12] = random();
        let nonce = Nonce::from_slice(nonce_bits.as_slice());

        let mut ciphertext = cipher.encrypt(nonce, msg).map_err(|_| Error::EncryptingErr)?;

        let mut result = Vec::new();

        result.extend_from_slice(&key_salt);
        result.extend_from_slice(&nonce_bits);
        result.append(&mut ciphertext);

        Ok(result)
    }

    /// Decrypts a given ciphertext with the given password.
    pub fn decrypt(ciphertext: &[u8], passwd: &[u8]) -> Result<Vec<u8>, Error> {
        let key_salt = &ciphertext[0..16];
        let argon2 = Argon2::default();

        let mut secret_passwd_hash: [u8; 32] = [0; 32];
        argon2.hash_password_into(passwd, key_salt, &mut secret_passwd_hash).map_err(|_| Error::DecryptingErr)?;
        let decryption_key = Key::from_slice(&secret_passwd_hash);

        let cipher = Aes256Gcm::new(decryption_key);

        let nonce_bits = &ciphertext[16..28];
        let nonce = Nonce::from_slice(nonce_bits);

        Ok(cipher.decrypt(nonce, &ciphertext[28..]).map_err(|_| Error::DecryptingErr)?)
    }

    #[cfg(test)]
    mod tests {
        use crate::network::crypt::{decrypt, encrypt};

        #[test]
        fn decrypting_encrypted_returns_original() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let ct = encrypt(msg, ps).unwrap();

            assert_eq!(decrypt(&ct, ps).unwrap(), msg);
        }

        #[test]
        fn encrypting_same_msg_with_same_password_returns_different_ciphertext() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let mut ciphertexts = Vec::new();

            for _ in 1..3 {
                let ct = encrypt(msg, ps).unwrap();
                assert!(!ciphertexts.contains(&ct));
                ciphertexts.push(ct);
            }
        }

        #[test]
        fn decrypting_with_incorrect_passwd_fails() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let ct = encrypt(msg, ps).unwrap();

            assert!(decrypt(&ct, b"Incorrect passwd").is_err());
        }

        #[test]
        fn decrypting_with_invalid_ciphertext_fails() {
            let msg = b"A message to keep secure.";
            let ps = b"Very secure passwd";

            let mut ct = encrypt(msg, ps).unwrap();
            ct.push(14);
            ct.push(36);
            ct.push(122);

            assert!(decrypt(&ct, ps).is_err());
        }
    }
}
