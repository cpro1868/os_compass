use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce, Key};
use rand::RngCore;
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub struct CryptoManager {
    cipher: Aes256Gcm,
}

impl CryptoManager {
    pub fn new(key: &[u8; 32]) -> Self {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(key);
        Self { cipher }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| e.to_string())?;
        
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);
        
        Ok(STANDARD.encode(&combined))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let combined = STANDARD.decode(encoded)
            .map_err(|e| e.to_string())?;
        
        if combined.len() < 12 {
            return Err("Invalid encrypted data".to_string());
        }
        
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| e.to_string())?;
        
        Ok(String::from_utf8_lossy(&plaintext).to_string())
    }
}

pub fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    key
}

pub fn key_to_base64(key: &[u8; 32]) -> String {
    STANDARD.encode(key)
}

pub fn key_from_base64(encoded: &str) -> Result<[u8; 32], String> {
    let bytes = STANDARD.decode(encoded)
        .map_err(|e| e.to_string())?;
    if bytes.len() != 32 {
        return Err("Invalid key length".to_string());
    }
    
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub fn derive_key_from_password(password: &str, salt: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    salt.hash(&mut hasher);
    let hash = hasher.finish();
    
    let mut key = [0u8; 32];
    key[..8].copy_from_slice(&hash.to_be_bytes());
    
    for i in 8..32 {
        key[i] = ((hash >> (i % 8)) as u8).wrapping_add(salt[i % salt.len()]);
    }
    
    key
}

use std::sync::Mutex;
use std::sync::MutexGuard;

lazy_static::lazy_static! {
    pub static ref CRYPTO: Mutex<Option<CryptoManager>> = Mutex::new(None);
}

pub fn init_crypto(key: [u8; 32]) {
    let mut guard: MutexGuard<Option<CryptoManager>> = CRYPTO.lock().unwrap();
    *guard = Some(CryptoManager::new(&key));
}

pub fn encrypt_string(plaintext: &str) -> Result<String, String> {
    let guard = CRYPTO.lock().map_err(|e| e.to_string())?;
    let crypto = guard.as_ref().ok_or("Crypto not initialized")?;
    crypto.encrypt(plaintext)
}

pub fn decrypt_string(encoded: &str) -> Result<String, String> {
    let guard = CRYPTO.lock().map_err(|e| e.to_string())?;
    let crypto = guard.as_ref().ok_or("Crypto not initialized")?;
    crypto.decrypt(encoded)
}
