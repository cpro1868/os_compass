use crate::crypto::{self, CRYPTO};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct CryptoStatus {
    pub initialized: bool,
    pub algorithm: String,
}

#[tauri::command]
pub fn crypto_status() -> CryptoStatus {
    let initialized = CRYPTO.lock().map(|guard| guard.is_some()).unwrap_or(false);
    CryptoStatus {
        initialized,
        algorithm: "AES-256-GCM".to_string(),
    }
}

#[tauri::command]
pub fn encrypt_data(plaintext: String) -> Result<String, String> {
    crypto::encrypt_string(&plaintext)
}

#[tauri::command]
pub fn decrypt_data(encoded: String) -> Result<String, String> {
    crypto::decrypt_string(&encoded)
}

#[derive(Debug, Deserialize)]
pub struct InitCryptoInput {
    pub key: String,
}

#[tauri::command]
pub fn init_crypto_with_key(input: InitCryptoInput) -> Result<(), String> {
    let key = crypto::key_from_base64(&input.key)?;
    crypto::init_crypto(key);
    Ok(())
}

#[tauri::command]
pub fn generate_crypto_key() -> String {
    let key = crypto::generate_key();
    crypto::key_to_base64(&key)
}

#[tauri::command]
pub fn derive_key(password: String, salt: String) -> String {
    let salt_bytes = salt.as_bytes();
    let key = crypto::derive_key_from_password(&password, salt_bytes);
    crypto::key_to_base64(&key)
}

#[tauri::command]
pub fn test_encryption() -> Result<String, String> {
    let test_plaintext = "Hello, OS-Compass! 你好，开源罗盘！";
    let encrypted = crypto::encrypt_string(test_plaintext)?;
    let decrypted = crypto::decrypt_string(&encrypted)?;
    
    if decrypted != test_plaintext {
        return Err("Decryption verification failed".to_string());
    }
    
    Ok(format!("OK - Original: {}, Encrypted: {}, Decrypted: {}", 
        test_plaintext, encrypted, decrypted))
}