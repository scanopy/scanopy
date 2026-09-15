use jsonwebtoken::{DecodingKey, EncodingKey};

/// The public key used to verify license JWTs.
/// Embedded at compile time from the PEM file in this directory.
const PUBLIC_KEY_PEM: &[u8] = include_bytes!("public_key.pem");

/// Get the decoding (verification) key for license JWTs.
pub fn decoding_key() -> DecodingKey {
    DecodingKey::from_ed_pem(PUBLIC_KEY_PEM).expect("Bundled public key must be valid Ed25519 PEM")
}

/// Parse an Ed25519 private signing key from PEM.
pub fn encoding_key_from_pem(key_pem: &str) -> anyhow::Result<EncodingKey> {
    Ok(EncodingKey::from_ed_pem(key_pem.as_bytes())?)
}

/// Load the private signing key from the `SCANOPY_LICENSE_SIGNING_KEY` env var.
/// Used by the license CLI tool. The cloud server reads the same variable
/// through `ServerConfig::license_signing_key`; it is never present on
/// customer servers.
pub fn encoding_key_from_env() -> anyhow::Result<EncodingKey> {
    let key_pem = std::env::var("SCANOPY_LICENSE_SIGNING_KEY")
        .map_err(|_| anyhow::anyhow!("SCANOPY_LICENSE_SIGNING_KEY env var not set"))?;
    encoding_key_from_pem(&key_pem)
}
