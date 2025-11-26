//! Authentication and encryption for app data
//!
//! This module provides password-based encryption for the app's configuration data.
//! It uses Argon2id for password hashing and key derivation, and AES-256-GCM for encryption.

use crate::{Error, Result};
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Size of the nonce for AES-GCM (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// Salt size for key derivation (separate from password hash salt)
const KEY_SALT_SIZE: usize = 32;

/// Encrypted file header to identify encrypted files
const ENCRYPTED_HEADER: &[u8] = b"PVMW_ENC_V1";

/// Authentication state stored on disk (password hash only, no sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    /// Version for migration support
    pub version: u32,
    /// Argon2 password hash (includes salt)
    pub password_hash: String,
    /// Salt for key derivation (base64 encoded)
    pub key_salt: String,
}

impl AuthState {
    /// Get the default auth state path
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("proxy-vm-wizard")
            .join("auth.json")
    }

    /// Check if authentication is set up
    pub fn is_setup() -> bool {
        Self::default_path().exists()
    }

    /// Load auth state from disk
    pub fn load() -> Result<Self> {
        let path = Self::default_path();
        if !path.exists() {
            return Err(Error::NotFound("Auth state not found".to_string()));
        }
        let content = fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Save auth state to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::default_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;
        Ok(())
    }

    /// Create a new auth state with the given password
    pub fn create(password: &str) -> Result<Self> {
        // Generate password hash
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| Error::Auth(format!("Failed to hash password: {}", e)))?
            .to_string();

        // Generate key derivation salt
        let mut key_salt = [0u8; KEY_SALT_SIZE];
        OsRng.fill(&mut key_salt);
        let key_salt = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key_salt);

        Ok(Self {
            version: 1,
            password_hash,
            key_salt,
        })
    }

    /// Verify a password against the stored hash
    pub fn verify_password(&self, password: &str) -> Result<bool> {
        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| Error::Auth(format!("Invalid password hash: {}", e)))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Derive an encryption key from the password
    pub fn derive_key(&self, password: &str) -> Result<[u8; 32]> {
        let key_salt =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &self.key_salt)
                .map_err(|e| Error::Auth(format!("Invalid key salt: {}", e)))?;

        let mut key = [0u8; 32];
        Argon2::default()
            .hash_password_into(password.as_bytes(), &key_salt, &mut key)
            .map_err(|e| Error::Auth(format!("Key derivation failed: {}", e)))?;

        Ok(key)
    }
}

/// Encryption manager for the application
#[derive(Clone)]
pub struct EncryptionManager {
    /// The derived encryption key
    key: [u8; 32],
}

impl EncryptionManager {
    /// Create a new encryption manager with the given key
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }

    /// Create from password and auth state
    pub fn from_password(password: &str, auth_state: &AuthState) -> Result<Self> {
        let key = auth_state.derive_key(password)?;
        Ok(Self::new(key))
    }

    /// Encrypt data
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| Error::Auth(format!("Failed to create cipher: {}", e)))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| Error::Auth(format!("Encryption failed: {}", e)))?;

        // Combine header + nonce + ciphertext
        let mut result = Vec::with_capacity(ENCRYPTED_HEADER.len() + NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(ENCRYPTED_HEADER);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt data
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Check header
        if data.len() < ENCRYPTED_HEADER.len() + NONCE_SIZE + 16 {
            return Err(Error::Auth("Invalid encrypted data: too short".to_string()));
        }

        if &data[..ENCRYPTED_HEADER.len()] != ENCRYPTED_HEADER {
            return Err(Error::Auth(
                "Invalid encrypted data: wrong header".to_string(),
            ));
        }

        let nonce_start = ENCRYPTED_HEADER.len();
        let ciphertext_start = nonce_start + NONCE_SIZE;

        let nonce = Nonce::from_slice(&data[nonce_start..ciphertext_start]);
        let ciphertext = &data[ciphertext_start..];

        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| Error::Auth(format!("Failed to create cipher: {}", e)))?;

        cipher.decrypt(nonce, ciphertext).map_err(|_| {
            Error::Auth("Decryption failed: wrong password or corrupted data".to_string())
        })
    }

    /// Check if data is encrypted (has the encrypted header)
    pub fn is_encrypted(data: &[u8]) -> bool {
        data.len() >= ENCRYPTED_HEADER.len() && &data[..ENCRYPTED_HEADER.len()] == ENCRYPTED_HEADER
    }

    /// Encrypt a string and return base64
    pub fn encrypt_string(&self, text: &str) -> Result<String> {
        let encrypted = self.encrypt(text.as_bytes())?;
        Ok(base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            encrypted,
        ))
    }

    /// Decrypt base64 data to string
    pub fn decrypt_string(&self, encrypted_base64: &str) -> Result<String> {
        let data =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encrypted_base64)
                .map_err(|e| Error::Auth(format!("Invalid base64: {}", e)))?;
        let decrypted = self.decrypt(&data)?;
        String::from_utf8(decrypted).map_err(|e| Error::Auth(format!("Invalid UTF-8: {}", e)))
    }

    /// Encrypt and write to file
    pub fn encrypt_to_file(&self, data: &[u8], path: &Path) -> Result<()> {
        let encrypted = self.encrypt(data)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, encrypted)?;
        Ok(())
    }

    /// Read and decrypt from file
    pub fn decrypt_from_file(&self, path: &Path) -> Result<Vec<u8>> {
        if !path.exists() {
            return Err(Error::NotFound(format!(
                "File not found: {}",
                path.display()
            )));
        }
        let data = fs::read(path)?;
        self.decrypt(&data)
    }

    /// Encrypt and write text to file
    pub fn encrypt_text_to_file(&self, text: &str, path: &Path) -> Result<()> {
        self.encrypt_to_file(text.as_bytes(), path)
    }

    /// Read and decrypt text from file
    pub fn decrypt_text_from_file(&self, path: &Path) -> Result<String> {
        let data = self.decrypt_from_file(path)?;
        String::from_utf8(data).map_err(|e| Error::Auth(format!("Invalid UTF-8: {}", e)))
    }
}

/// Check if a file is encrypted
pub fn is_file_encrypted(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let data = fs::read(path)?;
    Ok(EncryptionManager::is_encrypted(&data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_create_verify() {
        let password = "test_password_123";
        let auth = AuthState::create(password).unwrap();

        assert!(auth.verify_password(password).unwrap());
        assert!(!auth.verify_password("wrong_password").unwrap());
    }

    #[test]
    fn test_encryption_roundtrip() {
        let password = "test_password_123";
        let auth = AuthState::create(password).unwrap();
        let manager = EncryptionManager::from_password(password, &auth).unwrap();

        let original = b"Hello, World! This is secret data.";
        let encrypted = manager.encrypt(original).unwrap();
        let decrypted = manager.decrypt(&encrypted).unwrap();

        assert_eq!(original.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_encryption_string_roundtrip() {
        let password = "test_password_123";
        let auth = AuthState::create(password).unwrap();
        let manager = EncryptionManager::from_password(password, &auth).unwrap();

        let original = "Secret configuration data with special chars: é€🔐";
        let encrypted = manager.encrypt_string(original).unwrap();
        let decrypted = manager.decrypt_string(&encrypted).unwrap();

        assert_eq!(original, decrypted);
    }

    #[test]
    fn test_is_encrypted_check() {
        let password = "test_password_123";
        let auth = AuthState::create(password).unwrap();
        let manager = EncryptionManager::from_password(password, &auth).unwrap();

        let original = b"test data";
        let encrypted = manager.encrypt(original).unwrap();

        assert!(EncryptionManager::is_encrypted(&encrypted));
        assert!(!EncryptionManager::is_encrypted(original));
    }

    #[test]
    fn test_wrong_password_fails() {
        let password = "correct_password";
        let wrong_password = "wrong_password";
        let auth = AuthState::create(password).unwrap();

        let manager1 = EncryptionManager::from_password(password, &auth).unwrap();
        let manager2 = EncryptionManager::from_password(wrong_password, &auth).unwrap();

        let encrypted = manager1.encrypt(b"secret").unwrap();

        // Decryption with wrong key should fail
        assert!(manager2.decrypt(&encrypted).is_err());
    }
}
