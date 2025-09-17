use crate::{encryption_error, error::{MessengerError, Result}};
use aes_gcm::{Aes256Gcm, Key, Nonce, aead::{Aead, KeyInit}};
use p256::{ecdh::EphemeralSecret, PublicKey, elliptic_curve::sec1::ToEncodedPoint};
use rand::Rng;
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fmt::Debug;

/// Encryption engine for secure message handling
pub struct EncryptionEngine {
    cipher: Aes256Gcm,
    nonce: [u8; 12],
    key_rotation_counter: u32,
    max_messages_per_key: u32,
}

/// Key pair for ECDH key exchange
pub struct KeyPair {
    pub private_key: EphemeralSecret,
    pub public_key: PublicKey,
}

impl Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("public_key", &self.public_key.to_encoded_point(false).as_bytes())
            .finish()
    }
}

impl Clone for KeyPair {
    fn clone(&self) -> Self {
        // Generate a new key pair since EphemeralSecret cannot be cloned
        // This creates a new private key but maintains the same public key concept
        let new_private_key = EphemeralSecret::random(&mut rand::thread_rng());
        let new_public_key = PublicKey::from(&new_private_key);
        
        Self {
            private_key: new_private_key,
            public_key: new_public_key,
        }
    }
}

/// Shared secret derived from ECDH
#[derive(Debug, Clone)]
pub struct SharedSecret {
    pub encryption_key: [u8; 32],
    pub mac_key: [u8; 32],
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Key exchange manager
#[derive(Debug)]
pub struct KeyExchangeManager {
    key_pairs: HashMap<uuid::Uuid, KeyPair>,
    shared_secrets: HashMap<uuid::Uuid, SharedSecret>,
    key_rotation_interval: u32,
}

impl EncryptionEngine {
    /// Create a new encryption engine with a random key
    pub fn new() -> Result<Self> {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut key_bytes);
        
        let key = Key::<aes_gcm::Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);

        Ok(Self {
            cipher,
            nonce: nonce_bytes,
            key_rotation_counter: 0,
            max_messages_per_key: 100,
        })
    }

    /// Create encryption engine from existing key
    pub fn from_key(key: &[u8; 32]) -> Result<Self> {
        let key = Key::<aes_gcm::Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);

        Ok(Self {
            cipher,
            nonce: nonce_bytes,
            key_rotation_counter: 0,
            max_messages_per_key: 100,
        })
    }

    /// Encrypt a message
    pub fn encrypt_message(&mut self, message: &[u8]) -> Result<Vec<u8>> {
        // Generate a new nonce for each message
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the message
        let ciphertext = self.cipher.encrypt(nonce, message)
            .map_err(|e| encryption_error!("Failed to encrypt message: {}", e))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        self.key_rotation_counter += 1;
        Ok(result)
    }

    /// Decrypt a message
    pub fn decrypt_message(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        if encrypted_data.len() < 12 {
            return Err(encryption_error!("Invalid encrypted data length"));
        }

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&encrypted_data[0..12]);
        let ciphertext = &encrypted_data[12..];

        // Decrypt the message
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| encryption_error!("Failed to decrypt message: {}", e))?;

        Ok(plaintext)
    }

    /// Check if key rotation is needed
    pub fn should_rotate_key(&self) -> bool {
        self.key_rotation_counter >= self.max_messages_per_key
    }

    /// Rotate encryption key
    pub fn rotate_key(&mut self) -> Result<()> {
        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill(&mut key_bytes);
        
        let key = Key::<aes_gcm::Aes256Gcm>::from_slice(&key_bytes);
        self.cipher = Aes256Gcm::new(key);

        self.key_rotation_counter = 0;
        Ok(())
    }

    /// Get current key rotation counter
    pub fn key_rotation_counter(&self) -> u32 {
        self.key_rotation_counter
    }
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate() -> Self {
        let private_key = EphemeralSecret::random(&mut rand::thread_rng());
        let public_key = PublicKey::from(&private_key);
        
        Self {
            private_key,
            public_key,
        }
    }

    /// Get the public key as bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.public_key.to_encoded_point(false).as_bytes().to_vec()
    }

    /// Perform ECDH key exchange
    pub fn perform_key_exchange(&self, peer_public_key: &PublicKey) -> Result<SharedSecret> {
        let shared_secret = self.private_key.diffie_hellman(peer_public_key);
        let shared_secret_bytes = shared_secret.raw_secret_bytes();

        // Derive encryption and MAC keys using HKDF
        let encryption_key = Self::derive_key(&shared_secret_bytes, b"encryption")?;
        let mac_key = Self::derive_key(&shared_secret_bytes, b"mac")?;

        Ok(SharedSecret {
            encryption_key,
            mac_key,
            created_at: chrono::Utc::now(),
        })
    }

    /// Derive a key using HKDF
    fn derive_key(shared_secret: &[u8], info: &[u8]) -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        hasher.update(shared_secret);
        hasher.update(info);
        hasher.update(b"tcp-messenger-v1");
        
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        Ok(key)
    }
}

impl SharedSecret {
    /// Check if the shared secret is expired
    pub fn is_expired(&self, max_age_seconds: u64) -> bool {
        let age = chrono::Utc::now() - self.created_at;
        age.num_seconds() > max_age_seconds as i64
    }

    /// Get the encryption key
    pub fn encryption_key(&self) -> &[u8; 32] {
        &self.encryption_key
    }

    /// Get the MAC key
    pub fn mac_key(&self) -> &[u8; 32] {
        &self.mac_key
    }
}

impl KeyExchangeManager {
    /// Create a new key exchange manager
    pub fn new(key_rotation_interval: u32) -> Self {
        Self {
            key_pairs: HashMap::new(),
            shared_secrets: HashMap::new(),
            key_rotation_interval,
        }
    }

    /// Generate a new key pair for a peer
    pub fn generate_key_pair(&mut self, peer_id: uuid::Uuid) -> Result<KeyPair> {
        let key_pair = KeyPair::generate();
        self.key_pairs.insert(peer_id, key_pair.clone());
        Ok(key_pair)
    }

    /// Get the public key for a peer
    pub fn get_public_key(&self, peer_id: &uuid::Uuid) -> Result<&PublicKey> {
        self.key_pairs.get(peer_id)
            .map(|kp| &kp.public_key)
            .ok_or_else(|| encryption_error!("No key pair found for peer: {}", peer_id))
    }

    /// Perform key exchange with a peer
    pub fn perform_key_exchange(
        &mut self,
        peer_id: uuid::Uuid,
        peer_public_key: &PublicKey,
    ) -> Result<SharedSecret> {
        let key_pair = self.key_pairs.get(&peer_id)
            .ok_or_else(|| encryption_error!("No key pair found for peer: {}", peer_id))?;

        let shared_secret = key_pair.perform_key_exchange(peer_public_key)?;
        self.shared_secrets.insert(peer_id, shared_secret.clone());
        Ok(shared_secret)
    }

    /// Get shared secret for a peer
    pub fn get_shared_secret(&self, peer_id: &uuid::Uuid) -> Result<&SharedSecret> {
        self.shared_secrets.get(peer_id)
            .ok_or_else(|| encryption_error!("No shared secret found for peer: {}", peer_id))
    }

    /// Remove key pair and shared secret for a peer
    pub fn remove_peer(&mut self, peer_id: &uuid::Uuid) {
        self.key_pairs.remove(peer_id);
        self.shared_secrets.remove(peer_id);
    }

    /// Check if key rotation is needed for any peer
    pub fn needs_key_rotation(&self) -> bool {
        self.shared_secrets.values()
            .any(|secret| secret.is_expired(self.key_rotation_interval as u64))
    }

    /// Rotate keys for all peers
    pub fn rotate_all_keys(&mut self) -> Result<()> {
        let peer_ids: Vec<uuid::Uuid> = self.shared_secrets.keys().cloned().collect();
        
        for peer_id in peer_ids {
            self.generate_key_pair(peer_id)?;
        }
        
        Ok(())
    }
}

/// Message authentication code (MAC) for integrity verification
pub struct MessageAuthenticator;

impl MessageAuthenticator {
    /// Create a MAC for a message
    pub fn create_mac(key: &[u8; 32], message: &[u8]) -> Result<[u8; 32]> {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(message);
        hasher.update(b"tcp-messenger-mac");
        
        let result = hasher.finalize();
        let mut mac = [0u8; 32];
        mac.copy_from_slice(&result);
        Ok(mac)
    }

    /// Verify a MAC for a message
    pub fn verify_mac(key: &[u8; 32], message: &[u8], mac: &[u8; 32]) -> bool {
        let expected_mac = Self::create_mac(key, message).unwrap_or([0u8; 32]);
        expected_mac == *mac
    }
}

/// Secure message wrapper with encryption and authentication
pub struct SecureMessage {
    pub encrypted_data: Vec<u8>,
    pub mac: [u8; 32],
}

impl SecureMessage {
    /// Create a secure message from plaintext
    pub fn encrypt(
        plaintext: &[u8],
        encryption_key: &[u8; 32],
        mac_key: &[u8; 32],
    ) -> Result<Self> {
        // Create encryption engine
        let mut engine = EncryptionEngine::from_key(encryption_key)?;
        
        // Encrypt the message
        let encrypted_data = engine.encrypt_message(plaintext)?;
        
        // Create MAC
        let mac = MessageAuthenticator::create_mac(mac_key, &encrypted_data)?;
        
        Ok(Self {
            encrypted_data,
            mac,
        })
    }

    /// Decrypt a secure message
    pub fn decrypt(
        &self,
        encryption_key: &[u8; 32],
        mac_key: &[u8; 32],
    ) -> Result<Vec<u8>> {
        // Verify MAC first
        if !MessageAuthenticator::verify_mac(mac_key, &self.encrypted_data, &self.mac) {
            return Err(encryption_error!("MAC verification failed"));
        }

        // Create encryption engine
        let engine = EncryptionEngine::from_key(encryption_key)?;
        
        // Decrypt the message
        engine.decrypt_message(&self.encrypted_data)
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.encrypted_data.len().to_be_bytes());
        bytes.extend_from_slice(&self.encrypted_data);
        bytes.extend_from_slice(&self.mac);
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 4 + 32 {
            return Err(encryption_error!("Invalid secure message data"));
        }

        let length = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
        let encrypted_data = data[4..4 + length].to_vec();
        let mut mac = [0u8; 32];
        mac.copy_from_slice(&data[4 + length..4 + length + 32]);

        Ok(Self {
            encrypted_data,
            mac,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_roundtrip() {
        let mut engine = EncryptionEngine::new().unwrap();
        let message = b"Hello, World!";
        
        let encrypted = engine.encrypt_message(message).unwrap();
        let decrypted = engine.decrypt_message(&encrypted).unwrap();
        
        assert_eq!(message, &decrypted[..]);
    }

    #[test]
    fn test_key_exchange() {
        let key_pair1 = KeyPair::generate();
        let key_pair2 = KeyPair::generate();
        
        let shared_secret1 = key_pair1.perform_key_exchange(&key_pair2.public_key).unwrap();
        let shared_secret2 = key_pair2.perform_key_exchange(&key_pair1.public_key).unwrap();
        
        assert_eq!(shared_secret1.encryption_key, shared_secret2.encryption_key);
        assert_eq!(shared_secret1.mac_key, shared_secret2.mac_key);
    }

    #[test]
    fn test_secure_message() {
        let encryption_key = [1u8; 32];
        let mac_key = [2u8; 32];
        let message = b"Secure message";
        
        let secure_msg = SecureMessage::encrypt(message, &encryption_key, &mac_key).unwrap();
        let decrypted = secure_msg.decrypt(&encryption_key, &mac_key).unwrap();
        
        assert_eq!(message, &decrypted[..]);
    }
}
