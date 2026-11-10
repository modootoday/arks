//! OpenSSH `chacha20-poly1305@openssh.com` packet protection.
//!
//! This is the OpenSSH construction from `PROTOCOL.chacha20`, not RFC 8439
//! `ChaCha20Poly1305`: the 64-byte SSH key is split into a payload key and a
//! packet-length key, and every packet gets a one-time Poly1305 key from block
//! zero of the payload cipher.

use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
use chacha20::{ChaCha20Legacy, Key, LegacyNonce};
use poly1305::universal_hash::KeyInit;
use poly1305::{Key as Poly1305Key, Poly1305};
use std::io::{Read, Write};
use std::net::TcpStream;
use subtle::ConstantTimeEq;

const TAG_LEN: usize = 16;
const SSH_BLOCK_SIZE: usize = 8;

pub(crate) struct OpenSshChaCha {
    payload_key: [u8; 32],
    length_key: [u8; 32],
    sequence: u32,
}

impl OpenSshChaCha {
    pub(crate) fn new(key: &[u8], sequence: u32) -> Result<Self, String> {
        if key.len() != 64 {
            return Err(format!(
                "OpenSSH ChaCha key length {}, expected 64",
                key.len()
            ));
        }
        let mut payload_key = [0u8; 32];
        let mut length_key = [0u8; 32];
        payload_key.copy_from_slice(&key[..32]);
        length_key.copy_from_slice(&key[32..]);
        Ok(Self {
            payload_key,
            length_key,
            sequence,
        })
    }

    fn nonce(&self) -> [u8; 8] {
        // OpenSSH places the packet sequence number in network byte order in
        // the legacy ChaCha 64-bit nonce.
        u64::from(self.sequence).to_be_bytes()
    }

    fn crypt_length(&self, length: &mut [u8; 4]) {
        let mut cipher = ChaCha20Legacy::new(
            Key::from_slice(&self.length_key),
            LegacyNonce::from_slice(&self.nonce()),
        );
        cipher.apply_keystream(length);
    }

    fn poly_key_and_payload_cipher(&self) -> ([u8; 32], ChaCha20Legacy) {
        let mut cipher = ChaCha20Legacy::new(
            Key::from_slice(&self.payload_key),
            LegacyNonce::from_slice(&self.nonce()),
        );
        let mut poly_key = [0u8; 32];
        cipher.apply_keystream(&mut poly_key);
        // Payload encryption starts at block counter 1. The unused latter
        // half of block zero is skipped exactly as OpenSSH specifies.
        cipher.seek(64);
        (poly_key, cipher)
    }

    fn tag(poly_key: &[u8; 32], encrypted_packet: &[u8]) -> [u8; TAG_LEN] {
        let mac =
            Poly1305::new(Poly1305Key::from_slice(poly_key)).compute_unpadded(encrypted_packet);
        let mut out = [0u8; TAG_LEN];
        out.copy_from_slice(&mac);
        out
    }

    pub(crate) fn seal_packet(&mut self, payload: &[u8]) -> Result<Vec<u8>, String> {
        let mut padding_len = 4usize;
        // PROTOCOL.chacha20 encrypts the packet length separately. OpenSSH's
        // encrypted body (padding_length through padding) is block-aligned.
        while (1 + payload.len() + padding_len) % SSH_BLOCK_SIZE != 0 {
            padding_len += 1;
        }
        let packet_length = 1usize
            .checked_add(payload.len())
            .and_then(|n| n.checked_add(padding_len))
            .ok_or("SSH packet length overflow")?;
        let packet_length_u32 = u32::try_from(packet_length).map_err(|_| "SSH packet too large")?;

        let mut encrypted_length = packet_length_u32.to_be_bytes();
        self.crypt_length(&mut encrypted_length);

        let mut body = Vec::with_capacity(packet_length);
        body.push(padding_len as u8);
        body.extend_from_slice(payload);
        body.resize(packet_length, 0);
        let (poly_key, mut payload_cipher) = self.poly_key_and_payload_cipher();
        payload_cipher.apply_keystream(&mut body);

        let mut packet = Vec::with_capacity(4 + packet_length + TAG_LEN);
        packet.extend_from_slice(&encrypted_length);
        packet.extend_from_slice(&body);
        let tag = Self::tag(&poly_key, &packet);
        packet.extend_from_slice(&tag);
        self.sequence = self.sequence.wrapping_add(1);
        Ok(packet)
    }

    pub(crate) fn open_packet(
        &mut self,
        encrypted_length: [u8; 4],
        mut body: Vec<u8>,
        received_tag: [u8; TAG_LEN],
        max_payload: usize,
    ) -> Result<Vec<u8>, String> {
        let mut plain_length = encrypted_length;
        self.crypt_length(&mut plain_length);
        let packet_length = u32::from_be_bytes(plain_length) as usize;
        if packet_length < 5 || packet_length > max_payload.saturating_add(256) {
            return Err(format!("SSH encrypted packet length {packet_length}"));
        }
        if body.len() != packet_length {
            return Err("SSH encrypted packet body length mismatch".into());
        }

        let (poly_key, mut payload_cipher) = self.poly_key_and_payload_cipher();
        let mut authenticated = Vec::with_capacity(4 + body.len());
        authenticated.extend_from_slice(&encrypted_length);
        authenticated.extend_from_slice(&body);
        let expected_tag = Self::tag(&poly_key, &authenticated);
        if !bool::from(expected_tag.ct_eq(&received_tag)) {
            return Err("SSH ChaCha Poly1305 tag mismatch".into());
        }

        payload_cipher.apply_keystream(&mut body);
        let padding_len = body[0] as usize;
        if padding_len < 4 || padding_len + 1 > body.len() {
            return Err("SSH encrypted padding length invalid".into());
        }
        let payload_end = body.len() - padding_len;
        self.sequence = self.sequence.wrapping_add(1);
        Ok(body[1..payload_end].to_vec())
    }

    pub(crate) fn write_packet(
        &mut self,
        stream: &mut TcpStream,
        payload: &[u8],
    ) -> Result<(), String> {
        let packet = self.seal_packet(payload)?;
        stream.write_all(&packet).map_err(|e| e.to_string())?;
        // Git-over-SSH advertisements are small; without flush/Nagle-bypass the
        // peer can stall waiting for CHANNEL_DATA while we block on read.
        stream.flush().map_err(|e| e.to_string())
    }

    pub(crate) fn read_packet(
        &mut self,
        stream: &mut TcpStream,
        max_payload: usize,
    ) -> Result<Vec<u8>, String> {
        let mut encrypted_length = [0u8; 4];
        stream
            .read_exact(&mut encrypted_length)
            .map_err(|e| e.to_string())?;

        let mut plain_length = encrypted_length;
        self.crypt_length(&mut plain_length);
        let packet_length = u32::from_be_bytes(plain_length) as usize;
        if packet_length < 5 || packet_length > max_payload.saturating_add(256) {
            return Err(format!("SSH encrypted packet length {packet_length}"));
        }
        let mut body = vec![0u8; packet_length];
        stream.read_exact(&mut body).map_err(|e| e.to_string())?;
        let mut tag = [0u8; TAG_LEN];
        stream.read_exact(&mut tag).map_err(|e| e.to_string())?;
        self.open_packet(encrypted_length, body, tag, max_payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_roundtrip_and_length_is_encrypted() {
        let key = [0x5au8; 64];
        let mut sender = OpenSshChaCha::new(&key, 7).unwrap();
        let wire = sender.seal_packet(b"hello openssh").unwrap();
        assert_ne!(&wire[..4], &(24u32.to_be_bytes()));

        let mut receiver = OpenSshChaCha::new(&key, 7).unwrap();
        let encrypted_length: [u8; 4] = wire[..4].try_into().unwrap();
        let mut plain_length = encrypted_length;
        receiver.crypt_length(&mut plain_length);
        let body_len = u32::from_be_bytes(plain_length) as usize;
        let body = wire[4..4 + body_len].to_vec();
        let tag = wire[4 + body_len..].try_into().unwrap();
        let payload = receiver
            .open_packet(encrypted_length, body, tag, 1024)
            .unwrap();
        assert_eq!(payload, b"hello openssh");
    }

    #[test]
    fn packet_tag_rejects_tampering() {
        let key = [0x33u8; 64];
        let mut sender = OpenSshChaCha::new(&key, 0).unwrap();
        let mut wire = sender.seal_packet(b"authenticated").unwrap();
        wire[5] ^= 1;

        let mut receiver = OpenSshChaCha::new(&key, 0).unwrap();
        let encrypted_length = wire[..4].try_into().unwrap();
        let mut plain_length = encrypted_length;
        receiver.crypt_length(&mut plain_length);
        let body_len = u32::from_be_bytes(plain_length) as usize;
        let body = wire[4..4 + body_len].to_vec();
        let tag = wire[4 + body_len..].try_into().unwrap();
        assert!(
            receiver
                .open_packet(encrypted_length, body, tag, 1024)
                .is_err()
        );
    }
}
