//! SPAKE2 uses the Ed25519 group, not a home-grown PIN hash or a signing identity.
use crate::protocol::{self, Frame, Payload, VERSION};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload as AeadPayload},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use spake2::{Ed25519Group, Identity, Password, Spake2};

pub(crate) const AUTH_ERROR: &str = "无法验证连接，请核对 PIN 或稍后重试";
const CLIENT_ID: &[u8] = b"SonaHop/v2/client";
const HOST_ID: &[u8] = b"SonaHop/v2/host";
const MAX_PLAINTEXT: usize = 2800;
// Never allow a session to approach AES-GCM's per-key invocation limits.
const MAX_COUNTER: u64 = 1 << 32;

pub(crate) fn valid_pin(pin: &str) -> bool {
    pin.len() == 6 && pin.bytes().all(|byte| byte.is_ascii_digit())
}
pub(crate) fn generate_pin() -> Result<String, String> {
    loop {
        let mut bytes = [0; 4];
        getrandom::getrandom(&mut bytes).map_err(|_| "无法获取系统随机数".to_owned())?;
        let value = u32::from_be_bytes(bytes);
        if value < u32::MAX - u32::MAX % 1_000_000 {
            return Ok(format!("{:06}", value % 1_000_000));
        }
    }
}
pub(crate) fn client_start(pin: &str) -> (Spake2<Ed25519Group>, Vec<u8>) {
    Spake2::<Ed25519Group>::start_a(
        &Password::new(pin.as_bytes()),
        &Identity::new(CLIENT_ID),
        &Identity::new(HOST_ID),
    )
}
pub(crate) fn host_start(pin: &str) -> (Spake2<Ed25519Group>, Vec<u8>) {
    Spake2::<Ed25519Group>::start_b(
        &Password::new(pin.as_bytes()),
        &Identity::new(CLIENT_ID),
        &Identity::new(HOST_ID),
    )
}
pub(crate) fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}
pub(crate) fn decode(value: &str, maximum: usize) -> Result<Vec<u8>, String> {
    if value.len() > maximum.div_ceil(3) * 4 {
        return Err(AUTH_ERROR.into());
    }
    let bytes = STANDARD.decode(value).map_err(|_| AUTH_ERROR.to_owned())?;
    if bytes.len() > maximum {
        return Err(AUTH_ERROR.into());
    }
    Ok(bytes)
}

pub(crate) struct SecureChannel {
    send: DirectionalCipher,
    receive: DirectionalCipher,
    pub verification_code: String,
}
pub(crate) struct DirectionalCipher {
    cipher: Aes256Gcm,
    prefix: [u8; 4],
    counter: u64,
    binding: [u8; 32],
}
impl SecureChannel {
    pub fn establish(
        state: Spake2<Ed25519Group>,
        client_message: &[u8],
        host_message: &[u8],
        session_id: &str,
        client_name: &str,
        host_name: &str,
        host: bool,
    ) -> Result<Self, String> {
        let remote = if host { client_message } else { host_message };
        let mut secret = state.finish(remote).map_err(|_| AUTH_ERROR.to_owned())?;
        let mut transcript = Sha256::new();
        transcript.update(b"SonaHop SPAKE2 Ed25519 HKDF-SHA256 AES-256-GCM v2");
        for field in [
            session_id.as_bytes(),
            client_name.as_bytes(),
            host_name.as_bytes(),
            client_message,
            host_message,
        ] {
            transcript.update((field.len() as u32).to_be_bytes());
            transcript.update(field);
        }
        let binding: [u8; 32] = transcript.finalize().into();
        let hkdf = Hkdf::<Sha256>::new(Some(&binding), &secret);
        secret.fill(0);
        let mut material = [0; 76];
        hkdf.expand(
            b"SonaHop/v2/directional-keys-nonces-verification",
            &mut material,
        )
        .map_err(|_| AUTH_ERROR.to_owned())?;
        let client_key = &material[..32];
        let host_key = &material[32..64];
        let client_prefix: [u8; 4] = material[64..68].try_into().map_err(|_| AUTH_ERROR)?;
        let host_prefix: [u8; 4] = material[68..72].try_into().map_err(|_| AUTH_ERROR)?;
        let code: [u8; 4] = material[72..76].try_into().map_err(|_| AUTH_ERROR)?;
        let (send_key, receive_key, send_prefix, receive_prefix) = if host {
            (host_key, client_key, host_prefix, client_prefix)
        } else {
            (client_key, host_key, client_prefix, host_prefix)
        };
        let send = Aes256Gcm::new_from_slice(send_key).map_err(|_| AUTH_ERROR)?;
        let receive = Aes256Gcm::new_from_slice(receive_key).map_err(|_| AUTH_ERROR)?;
        material.fill(0);
        Ok(Self {
            send: DirectionalCipher {
                cipher: send,
                prefix: send_prefix,
                counter: 0,
                binding,
            },
            receive: DirectionalCipher {
                cipher: receive,
                prefix: receive_prefix,
                counter: 0,
                binding,
            },
            verification_code: format!("{:06}", u32::from_be_bytes(code) % 1_000_000),
        })
    }
    pub fn seal(&mut self, payload: &Payload) -> Result<Frame, String> {
        self.send.seal(payload)
    }
    pub fn open(&mut self, frame: Frame) -> Result<Payload, String> {
        self.receive.open(frame)
    }
    pub fn session_tag(&self) -> String {
        // Public handshake transcript digest; never key or PIN material.
        self.send.binding[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
    pub fn into_split(self) -> (DirectionalCipher, DirectionalCipher) {
        (self.send, self.receive)
    }
}
impl DirectionalCipher {
    fn nonce(&self, counter: u64) -> [u8; 12] {
        let mut nonce = [0; 12];
        nonce[..4].copy_from_slice(&self.prefix);
        nonce[4..].copy_from_slice(&counter.to_be_bytes());
        nonce
    }
    fn aad(&self, counter: u64) -> Vec<u8> {
        let mut aad = Vec::with_capacity(45);
        aad.push(VERSION);
        aad.extend_from_slice(&self.binding);
        aad.extend_from_slice(&self.prefix);
        aad.extend_from_slice(&counter.to_be_bytes());
        aad
    }
    pub fn seal(&mut self, payload: &Payload) -> Result<Frame, String> {
        if self.counter >= MAX_COUNTER {
            return Err("加密会话达到帧数上限，请重新连接".into());
        }
        let plaintext = protocol::bounded_json(payload, MAX_PLAINTEXT)?;
        let counter = self.counter;
        let nonce = self.nonce(counter);
        let aad = self.aad(counter);
        let ciphertext = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                AeadPayload {
                    msg: &plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| AUTH_ERROR.to_owned())?;
        self.counter += 1;
        Ok(Frame::Encrypted {
            version: VERSION,
            counter,
            body: encode(&ciphertext),
        })
    }
    pub fn open(&mut self, frame: Frame) -> Result<Payload, String> {
        let Frame::Encrypted {
            version: VERSION,
            counter,
            body,
        } = frame
        else {
            return Err(AUTH_ERROR.into());
        };
        if counter != self.counter || counter >= MAX_COUNTER {
            return Err(AUTH_ERROR.into());
        }
        let ciphertext = decode(&body, MAX_PLAINTEXT + 16)?;
        let nonce = self.nonce(counter);
        let aad = self.aad(counter);
        let plaintext = self
            .cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                AeadPayload {
                    msg: &ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| AUTH_ERROR.to_owned())?;
        let payload = serde_json::from_slice(&plaintext).map_err(|_| AUTH_ERROR.to_owned())?;
        self.counter += 1;
        Ok(payload)
    }
}
