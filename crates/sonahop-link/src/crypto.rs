//! Fresh anonymous X25519 keys bind each encrypted v3 session to its handshake.
use crate::protocol::{self, Frame, Payload, VERSION};
use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload as AeadPayload},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use hkdf::Hkdf;
use sha2::{Digest, Sha256};
use x25519_dalek::{EphemeralSecret, PublicKey};

pub(crate) const AUTH_ERROR: &str = "无法建立加密连接，请稍后重试";
const MAX_PLAINTEXT: usize = 2800;
// Never allow a session to approach AES-GCM's per-key invocation limits.
const MAX_COUNTER: u64 = 1 << 32;

pub(crate) fn ephemeral_start() -> (EphemeralSecret, [u8; 32]) {
    let secret = EphemeralSecret::random();
    let public = PublicKey::from(&secret).to_bytes();
    (secret, public)
}
pub(crate) fn decode_public_key(value: &str) -> Result<[u8; 32], String> {
    decode(value, 32)?.try_into().map_err(|_| AUTH_ERROR.into())
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
}
pub(crate) struct DirectionalCipher {
    cipher: Aes256Gcm,
    prefix: [u8; 4],
    counter: u64,
    binding: [u8; 32],
}
impl SecureChannel {
    pub fn establish(
        state: EphemeralSecret,
        client_message: &[u8; 32],
        host_message: &[u8; 32],
        session_id: &str,
        client_name: &str,
        host_name: &str,
        host: bool,
    ) -> Result<Self, String> {
        let (local, remote) = if host {
            (host_message, client_message)
        } else {
            (client_message, host_message)
        };
        if PublicKey::from(&state).as_bytes() != local {
            return Err(AUTH_ERROR.into());
        }
        let secret = state.diffie_hellman(&PublicKey::from(*remote));
        if !secret.was_contributory() {
            return Err(AUTH_ERROR.into());
        }
        let mut transcript = Sha256::new();
        transcript.update(b"SonaHop X25519 HKDF-SHA256 AES-256-GCM v3");
        for field in [
            session_id.as_bytes(),
            client_name.as_bytes(),
            host_name.as_bytes(),
            client_message.as_slice(),
            host_message.as_slice(),
        ] {
            transcript.update((field.len() as u32).to_be_bytes());
            transcript.update(field);
        }
        let binding: [u8; 32] = transcript.finalize().into();
        let hkdf = Hkdf::<Sha256>::new(Some(&binding), secret.as_bytes());
        drop(secret);
        let mut material = [0; 72];
        hkdf.expand(b"SonaHop/v3/directional-keys-nonces", &mut material)
            .map_err(|_| AUTH_ERROR.to_owned())?;
        let client_key = &material[..32];
        let host_key = &material[32..64];
        let client_prefix: [u8; 4] = material[64..68].try_into().map_err(|_| AUTH_ERROR)?;
        let host_prefix: [u8; 4] = material[68..72].try_into().map_err(|_| AUTH_ERROR)?;
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
        })
    }
    pub fn seal(&mut self, payload: &Payload) -> Result<Frame, String> {
        self.send.seal(payload)
    }
    pub fn open(&mut self, frame: Frame) -> Result<Payload, String> {
        self.receive.open(frame)
    }
    pub fn session_tag(&self) -> String {
        // Public handshake transcript digest; never secret key material.
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

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION_ID: &str = "00000000-0000-4000-8000-000000000001";

    fn channels(host_name_seen_by_client: &str) -> (SecureChannel, SecureChannel) {
        let (client, client_key) = ephemeral_start();
        let (host, host_key) = ephemeral_start();
        let client = SecureChannel::establish(
            client,
            &client_key,
            &host_key,
            SESSION_ID,
            "player",
            host_name_seen_by_client,
            false,
        )
        .unwrap();
        let host = SecureChannel::establish(
            host,
            &client_key,
            &host_key,
            SESSION_ID,
            "player",
            "host",
            true,
        )
        .unwrap();
        (client, host)
    }

    fn copy_frame(frame: &Frame) -> Frame {
        serde_json::from_slice(&serde_json::to_vec(frame).unwrap()).unwrap()
    }

    #[test]
    fn ephemeral_public_keys_are_fresh_and_exactly_sized() {
        let (_, first) = ephemeral_start();
        let (_, second) = ephemeral_start();
        assert_ne!(first, second);
        assert_eq!(decode_public_key(&encode(&first)).unwrap(), first);
        for length in [0, 1, 31, 33, 64] {
            assert!(decode_public_key(&encode(&vec![9; length])).is_err());
        }
        assert!(decode_public_key("invalid base64!").is_err());
    }

    #[test]
    fn zero_and_low_order_public_keys_are_rejected() {
        for remote in [[0; 32], {
            let mut key = [0; 32];
            key[0] = 1;
            key
        }] {
            let (secret, local) = ephemeral_start();
            assert!(
                SecureChannel::establish(
                    secret, &local, &remote, SESSION_ID, "player", "host", false,
                )
                .is_err()
            );
        }
    }

    #[test]
    fn both_directions_confirm_and_exchange_encrypted_payloads() {
        let (mut client, mut host) = channels("host");
        assert_eq!(client.session_tag(), host.session_tag());
        let confirm = client
            .seal(&Payload::Confirm {
                id: SESSION_ID.into(),
            })
            .unwrap();
        assert!(matches!(host.open(confirm).unwrap(), Payload::Confirm { id } if id == SESSION_ID));
        let accepted = host
            .seal(&Payload::Accepted {
                id: SESSION_ID.into(),
                host_name: "host".into(),
            })
            .unwrap();
        assert!(
            matches!(client.open(accepted).unwrap(), Payload::Accepted { id, host_name } if id == SESSION_ID && host_name == "host")
        );
        let control = client
            .seal(&Payload::Control {
                payload: serde_json::json!({"kind":"start"}),
            })
            .unwrap();
        assert!(
            matches!(host.open(control).unwrap(), Payload::Control { payload } if payload["kind"] == "start")
        );
        let media = host
            .seal(&Payload::Media {
                payload: encode(&[1, 2, 3]),
            })
            .unwrap();
        assert!(
            matches!(client.open(media).unwrap(), Payload::Media { payload } if decode(&payload, 3).unwrap() == [1, 2, 3])
        );
    }

    #[test]
    fn replay_and_reflection_are_rejected_without_consuming_the_next_counter() {
        let (mut client, mut host) = channels("host");
        let first = client.seal(&Payload::Ping).unwrap();
        let replay = copy_frame(&first);
        let reflection = copy_frame(&first);
        assert!(client.open(reflection).is_err());
        assert!(matches!(host.open(first).unwrap(), Payload::Ping));
        assert!(host.open(replay).is_err());
        let next = client.seal(&Payload::Pong).unwrap();
        assert!(matches!(host.open(next).unwrap(), Payload::Pong));
        let response = host.seal(&Payload::Pong).unwrap();
        assert!(matches!(client.open(response).unwrap(), Payload::Pong));
    }

    #[test]
    fn tampered_ciphertext_and_counter_are_rejected() {
        let (mut client, mut host) = channels("host");
        let original = client.seal(&Payload::Ping).unwrap();
        let mut modified = copy_frame(&original);
        let Frame::Encrypted { body, .. } = &mut modified else {
            unreachable!()
        };
        let mut bytes = decode(body, MAX_PLAINTEXT + 16).unwrap();
        bytes[0] ^= 1;
        *body = encode(&bytes);
        assert!(host.open(modified).is_err());
        let mut wrong_counter = copy_frame(&original);
        let Frame::Encrypted { counter, .. } = &mut wrong_counter else {
            unreachable!()
        };
        *counter += 1;
        assert!(host.open(wrong_counter).is_err());
        assert!(matches!(host.open(original).unwrap(), Payload::Ping));
    }

    #[test]
    fn changed_transcript_and_cross_session_frames_are_rejected() {
        let (mut mismatched_client, mut mismatched_host) = channels("another host");
        let frame = mismatched_client.seal(&Payload::Ping).unwrap();
        assert!(mismatched_host.open(frame).is_err());
        let (mut first_client, _) = channels("host");
        let (_, mut another_host) = channels("host");
        let frame = first_client.seal(&Payload::Ping).unwrap();
        assert!(another_host.open(frame).is_err());
    }
}
