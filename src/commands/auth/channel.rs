//! Noise handles key agreement, transcript binding and authenticated encryption.
use anyhow::{bail, Result};
const PATTERN: &str = "Noise_XX_25519_ChaChaPoly_SHA256";
pub struct Handshake(snow::HandshakeState);
pub struct Channel {
    state: snow::TransportState,
    code: String,
}
impl Handshake {
    pub fn new(session: &str, initiator: bool) -> Result<Self> {
        let builder = snow::Builder::new(PATTERN.parse()?);
        let pair = builder.generate_keypair()?;
        let prologue = format!("envx-auth-pairing-v1:{session}");
        let builder = builder
            .local_private_key(&pair.private)?
            .prologue(prologue.as_bytes())?;
        Ok(Self(if initiator {
            builder.build_initiator()?
        } else {
            builder.build_responder()?
        }))
    }
    pub fn write(&mut self) -> Result<String> {
        let mut output = [0u8; 1024];
        let len = self.0.write_message(&[], &mut output)?;
        Ok(hex::encode(&output[..len]))
    }
    pub fn read(&mut self, message: &str) -> Result<()> {
        if message.len() > 2048 {
            bail!("Oversized pairing handshake");
        }
        let mut output = [0u8; 1024];
        if self.0.read_message(&hex::decode(message)?, &mut output)? != 0 {
            bail!("Unexpected handshake payload");
        }
        Ok(())
    }
    pub fn finish(self) -> Result<Channel> {
        if !self.0.is_handshake_finished() {
            bail!("Incomplete handshake");
        }
        // Compare the complete transcript hash, not a grindable short numeric code.
        let code = hex::encode(self.0.get_handshake_hash());
        Ok(Channel {
            state: self.0.into_transport_mode()?,
            code,
        })
    }
}
impl Channel {
    pub fn code(&self) -> &str {
        &self.code
    }
    pub fn encrypt(&mut self, plaintext: &[u8]) -> Result<String> {
        if plaintext.len() > 65000 {
            bail!("Identity is too large to transfer");
        }
        let mut output = vec![0u8; 65535];
        let len = self.state.write_message(plaintext, &mut output)?;
        Ok(hex::encode(&output[..len]))
    }
    pub fn decrypt(&mut self, ciphertext: &str) -> Result<Vec<u8>> {
        if ciphertext.len() > 131070 {
            bail!("Oversized pairing payload");
        }
        let mut output = vec![0u8; 65535];
        let len = self
            .state
            .read_message(&hex::decode(ciphertext)?, &mut output)?;
        output.truncate(len);
        Ok(output)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn handshake(id: &str) -> (Channel, Channel) {
        let mut a = Handshake::new(id, true).unwrap();
        let mut b = Handshake::new(id, false).unwrap();
        b.read(&a.write().unwrap()).unwrap();
        a.read(&b.write().unwrap()).unwrap();
        b.read(&a.write().unwrap()).unwrap();
        (a.finish().unwrap(), b.finish().unwrap())
    }
    #[test]
    fn authenticated_transport_and_peer_substitution() {
        let (mut a, mut b) = handshake("session-one");
        assert_eq!(a.code(), b.code());
        let (c, _) = handshake("session-one");
        assert_ne!(a.code(), c.code());
        let ciphertext = a.encrypt(b"synthetic identity").unwrap();
        assert!(!ciphertext.contains("synthetic identity"));
        assert_eq!(b.decrypt(&ciphertext).unwrap(), b"synthetic identity");
        assert!(b.decrypt(&ciphertext).is_err());
        let mut wrong = Handshake::new("other-session", false).unwrap();
        let mut source = Handshake::new("session", true).unwrap();
        wrong.read(&source.write().unwrap()).unwrap();
        assert!(source.read(&wrong.write().unwrap()).is_err());
    }
    #[test]
    fn modified_ciphertext_fails_closed() {
        let (mut a, mut b) = handshake("session");
        let encrypted = a.encrypt(b"synthetic identity").unwrap();
        let mut bytes = hex::decode(encrypted).unwrap();
        bytes[0] ^= 1;
        assert!(b.decrypt(&hex::encode(bytes)).is_err());
    }
}
