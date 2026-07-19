use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};

fn main() {
    let key_base64 = "uHP+p5NTNaUwPusMjuoMKzxWqSrzxbr8O/ngbiOOGA0=";
    let encrypted_api_key = "QCVHf++tA2wMU8nKlLwDUo+CLunZTpjKaGj44J2Co7Yol7kvJBAn5rspr9YYY5oXr9JHWwItWMu86HFotMK/CRME3/dzRDrsvbWSjY5cVA==";

    let key_bytes = STANDARD.decode(key_base64).unwrap();
    println!("Key length: {}", key_bytes.len());

    let cipher = Aes256Gcm::new_from_slice(&key_bytes).unwrap();

    let combined = STANDARD.decode(encrypted_api_key).unwrap();
    println!("Encrypted data length: {}", combined.len());

    if combined.len() < 12 {
        println!("Data too short");
        return;
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    println!("Nonce: {:?}", nonce_bytes);
    println!("Ciphertext length: {}", ciphertext.len());

    match cipher.decrypt(nonce, ciphertext) {
        Ok(plaintext) => {
            println!("Decrypted successfully!");
            println!("Plaintext: {}", String::from_utf8_lossy(&plaintext));
        }
        Err(e) => {
            println!("Decrypt failed: {:?}", e);
        }
    }
}
