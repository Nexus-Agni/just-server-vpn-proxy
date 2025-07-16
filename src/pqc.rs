use pqcrypto_kyber::kyber768;
use pqcrypto_dilithium::dilithium3;
use pqcrypto_sphincsplus::sphincssha2128ssimple;
use pqcrypto_traits::kem::{PublicKey as KemPublicKey, SecretKey as KemSecretKey, SharedSecret, Ciphertext};
use pqcrypto_traits::sign::{PublicKey as SignPublicKey, SecretKey as SignSecretKey, SignedMessage};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqcKeyPair {
    pub kyber_public_key: String,
    pub kyber_secret_key: String,
    pub dilithium_public_key: String,
    pub dilithium_secret_key: String,
    pub sphincs_public_key: String,
    pub sphincs_secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PqcSharedData {
    pub ciphertext: String,
    pub shared_secret: String,
    pub signature: String,
}

pub struct PqcCrypto {
    pub key_pair: PqcKeyPair,
}

impl PqcCrypto {
    /// Initialize a new PQC instance with generated key pairs
    pub fn new() -> Self {
        let (kyber_pk, kyber_sk) = kyber768::keypair();
        let (dilithium_pk, dilithium_sk) = dilithium3::keypair();
        let (sphincs_pk, sphincs_sk) = sphincssha2128ssimple::keypair();

        let key_pair = PqcKeyPair {
            kyber_public_key: BASE64.encode(kyber_pk.as_bytes()),
            kyber_secret_key: BASE64.encode(kyber_sk.as_bytes()),
            dilithium_public_key: BASE64.encode(dilithium_pk.as_bytes()),
            dilithium_secret_key: BASE64.encode(dilithium_sk.as_bytes()),
            sphincs_public_key: BASE64.encode(sphincs_pk.as_bytes()),
            sphincs_secret_key: BASE64.encode(sphincs_sk.as_bytes()),
        };

        Self { key_pair }
    }

    /// Load PQC instance from existing key pairs
    pub fn from_keys(key_pair: PqcKeyPair) -> Self {
        Self { key_pair }
    }

    /// Perform Kyber key encapsulation (replaces RSA/ECDSA key exchange)
    pub fn kyber_encapsulate(&self, peer_public_key: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
        let peer_pk_bytes = BASE64.decode(peer_public_key)?;
        let peer_pk = kyber768::PublicKey::from_bytes(&peer_pk_bytes)?;
        
        let (shared_secret, ciphertext) = kyber768::encapsulate(&peer_pk);
        
        Ok((
            BASE64.encode(shared_secret.as_bytes()),
            BASE64.encode(ciphertext.as_bytes())
        ))
    }

    /// Perform Kyber key decapsulation
    pub fn kyber_decapsulate(&self, ciphertext: &str) -> Result<String, Box<dyn std::error::Error>> {
        let sk_bytes = BASE64.decode(&self.key_pair.kyber_secret_key)?;
        let sk = kyber768::SecretKey::from_bytes(&sk_bytes)?;
        
        let ct_bytes = BASE64.decode(ciphertext)?;
        let ct = kyber768::Ciphertext::from_bytes(&ct_bytes)?;
        
        let shared_secret = kyber768::decapsulate(&ct, &sk);
        
        Ok(BASE64.encode(shared_secret.as_bytes()))
    }

    /// Create a Dilithium signature (replaces traditional digital signatures)
    pub fn dilithium_sign(&self, message: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let sk_bytes = BASE64.decode(&self.key_pair.dilithium_secret_key)?;
        let sk = dilithium3::SecretKey::from_bytes(&sk_bytes)?;
        
        let signed_message = dilithium3::sign(message, &sk);
        
        Ok(BASE64.encode(signed_message.as_bytes()))
    }

    /// Verify a Dilithium signature  
    pub fn dilithium_verify(&self, signed_message_b64: &str, public_key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let pk_bytes = BASE64.decode(public_key)?;
        let pk = dilithium3::PublicKey::from_bytes(&pk_bytes)?;
        
        let signed_bytes = BASE64.decode(signed_message_b64)?;
        let signed_msg = dilithium3::SignedMessage::from_bytes(&signed_bytes)?;
        
        let verified_message = dilithium3::open(&signed_msg, &pk)?;
        
        Ok(verified_message)
    }

    /// Create a SPHINCS+ signature (alternative signature scheme)
    pub fn sphincs_sign(&self, message: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
        let sk_bytes = BASE64.decode(&self.key_pair.sphincs_secret_key)?;
        let sk = sphincssha2128ssimple::SecretKey::from_bytes(&sk_bytes)?;
        
        let signed_message = sphincssha2128ssimple::sign(message, &sk);
        
        Ok(BASE64.encode(signed_message.as_bytes()))
    }

    /// Verify a SPHINCS+ signature
    pub fn sphincs_verify(&self, signed_message_b64: &str, public_key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let pk_bytes = BASE64.decode(public_key)?;
        let pk = sphincssha2128ssimple::PublicKey::from_bytes(&pk_bytes)?;
        
        let signed_bytes = BASE64.decode(signed_message_b64)?;
        let signed_msg = sphincssha2128ssimple::SignedMessage::from_bytes(&signed_bytes)?;
        
        let verified_message = sphincssha2128ssimple::open(&signed_msg, &pk)?;
        
        Ok(verified_message)
    }

    /// Symmetric encryption using shared secret (replaces AES)
    /// This is a simple XOR-based encryption for demonstration
    /// In production, you'd want to use a proper AEAD cipher with the shared secret as key
    pub fn symmetric_encrypt(&self, data: &[u8], shared_secret: &str) -> Result<String, Box<dyn std::error::Error>> {
        let key = BASE64.decode(shared_secret)?;
        let mut encrypted = Vec::new();
        
        for (i, &byte) in data.iter().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }
        
        Ok(BASE64.encode(&encrypted))
    }

    /// Symmetric decryption using shared secret
    pub fn symmetric_decrypt(&self, encrypted_data: &str, shared_secret: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let key = BASE64.decode(shared_secret)?;
        let encrypted = BASE64.decode(encrypted_data)?;
        let mut decrypted = Vec::new();
        
        for (i, &byte) in encrypted.iter().enumerate() {
            decrypted.push(byte ^ key[i % key.len()]);
        }
        
        Ok(decrypted)
    }

    /// Generate a hash using SHA-3 (quantum-resistant alternative to SHA-256/384)
    pub fn hash_data(&self, data: &[u8]) -> String {
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        let result = hasher.finalize();
        BASE64.encode(result)
    }

    /// Get public keys for sharing
    pub fn get_public_keys(&self) -> (String, String, String) {
        (
            self.key_pair.kyber_public_key.clone(),
            self.key_pair.dilithium_public_key.clone(),
            self.key_pair.sphincs_public_key.clone(),
        )
    }

    /// Create a secure session with another party
    pub fn create_secure_session(&self, peer_kyber_pk: &str) -> Result<PqcSharedData, Box<dyn std::error::Error>> {
        // 1. Perform key encapsulation
        let (shared_secret, ciphertext) = self.kyber_encapsulate(peer_kyber_pk)?;
        
        // 2. Create a signature of the shared secret for authentication
        let signature = self.dilithium_sign(shared_secret.as_bytes())?;
        
        Ok(PqcSharedData {
            ciphertext,
            shared_secret,
            signature,
        })
    }

    /// Verify and establish secure session
    pub fn verify_secure_session(&self, session_data: &PqcSharedData, peer_dilithium_pk: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 1. Decapsulate to get shared secret
        let shared_secret = self.kyber_decapsulate(&session_data.ciphertext)?;
        
        // 2. Verify the signature
        let verified_message = self.dilithium_verify(&session_data.signature, peer_dilithium_pk)?;
        
        // 3. Check if the verified message matches the shared secret
        if verified_message == shared_secret.as_bytes() {
            Ok(shared_secret)
        } else {
            Err("Signature verification failed".into())
        }
    }
}

impl Default for PqcCrypto {
    fn default() -> Self {
        Self::new()
    }
}
