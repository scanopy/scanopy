//! Operator-supplied CA certificates trusted for outbound TLS, in addition to the bundled
//! webpki roots.
//!
//! Self-hosters who run a private CA point `SCANOPY_TRUSTED_CA_BUNDLE` (server and daemon) at a
//! PEM file. It is read and validated once at startup, and every client that talks to an
//! operator-controlled host (OIDC issuer, SMTP relay, daemon, license server, UniFi controller)
//! adds these certificates to its root store. A set-but-unusable file stops startup rather than
//! leaving those calls to fail later with an opaque TLS error.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use lettre::transport::smtp::client::{Certificate as SmtpCertificate, TlsParameters};
use rustls::RootCertStore;
use rustls::pki_types::{CertificateDer, pem::PemObject};

pub struct TrustedCaBundle {
    path: PathBuf,
    certs: Vec<CertificateDer<'static>>,
}

impl std::fmt::Debug for TrustedCaBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrustedCaBundle")
            .field("path", &self.path)
            .field("certificates", &self.certs.len())
            .finish()
    }
}

impl TrustedCaBundle {
    /// Read and validate the bundle at `path`. Every error names the path.
    pub fn load(path: &Path) -> Result<Self> {
        let pem = std::fs::read(path)
            .with_context(|| format!("Could not read trusted CA bundle {}", path.display()))?;
        let certs = Self::parse(&pem)
            .with_context(|| format!("Invalid trusted CA bundle {}", path.display()))?;
        Ok(Self {
            path: path.to_path_buf(),
            certs,
        })
    }

    /// Parse every `CERTIFICATE` block in `pem` and check each is usable as a trust anchor.
    /// Fails on malformed PEM, on a certificate rustls can't use, and on a file with none.
    fn parse(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>> {
        let certs = CertificateDer::pem_slice_iter(pem)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("malformed PEM: {e}"))?;
        if certs.is_empty() {
            bail!("no PEM certificates found");
        }
        let mut store = RootCertStore::empty();
        for (i, cert) in certs.iter().enumerate() {
            store
                .add(cert.clone())
                .map_err(|e| anyhow!("certificate {} is not a usable CA: {e}", i + 1))?;
        }
        Ok(certs)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn len(&self) -> usize {
        self.certs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.certs.is_empty()
    }

    /// Add an optional bundle to a reqwest client's roots; the bundled webpki roots stay
    /// enabled. `None` leaves the builder untouched.
    pub fn apply(bundle: Option<&Self>, builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
        let Some(bundle) = bundle else {
            return builder;
        };
        bundle.certs.iter().fold(builder, |builder, cert| {
            builder.add_root_certificate(
                reqwest::Certificate::from_der(cert)
                    .expect("reqwest's rustls Certificate::from_der only stores the bytes"),
            )
        })
    }

    /// TLS parameters for an SMTP connection to `domain`: the default roots plus the bundle.
    pub fn smtp_tls_parameters(&self, domain: String) -> Result<TlsParameters> {
        self.certs
            .iter()
            .fold(TlsParameters::builder(domain), |builder, cert| {
                builder.add_root_certificate(
                    SmtpCertificate::from_der(cert.to_vec())
                        .expect("lettre's rustls-only Certificate::from_der does not fail"),
                )
            })
            .build()
            .map_err(|e| anyhow!("Failed to build SMTP TLS parameters: {e}"))
    }
}

/// True when `err`, or anything in its source chain, is rustls rejecting the peer's
/// certificate (unknown issuer, expired, wrong name, ...). Clients use it to report an
/// untrusted certificate as such, instead of as an unreachable host, and to point at
/// `SCANOPY_TRUSTED_CA_BUNDLE`. rustls errors reach callers wrapped in `io::Error`, whose
/// payload is only reachable through `get_ref`, so both links are followed.
pub fn is_untrusted_certificate(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut current = Some(err);
    while let Some(e) = current {
        if matches!(
            e.downcast_ref::<rustls::Error>(),
            Some(rustls::Error::InvalidCertificate(_))
        ) {
            return true;
        }
        if let Some(inner) = e
            .downcast_ref::<std::io::Error>()
            .and_then(|io| io.get_ref())
            && is_untrusted_certificate(inner)
        {
            return true;
        }
        current = e.source();
    }
    false
}

/// Test fixtures: throwaway CAs and a local HTTPS server whose leaf is signed by CA A.
#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Arc;

    use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};

    // Self-signed P-256 CAs (100-year validity) and a `localhost` / 127.0.0.1 leaf signed by
    // CA A. Test-only material, never trusted anywhere else.
    pub(crate) const CA_A: &str = "-----BEGIN CERTIFICATE-----
MIIBjjCCATWgAwIBAgIUIegRP0BDvLFpg3KGCo/01+uEpmkwCgYIKoZIzj0EAwIw
HDEaMBgGA1UEAwwRU2Nhbm9weSBUZXN0IENBIGEwIBcNMjYwOTI4MjEwNjA0WhgP
MjEyNjA5MDQyMTA2MDRaMBwxGjAYBgNVBAMMEVNjYW5vcHkgVGVzdCBDQSBhMFkw
EwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEQOQjysgGHjjCwpXjlnwYFXmeUgCMfwZ7
O05pxTJTGckG2H83Iz0bxT1RQ4hptichs2zPUGrgbSauBSO51/lyaaNTMFEwHQYD
VR0OBBYEFDrWuuI5jajPLQYnK/PQcD5uN03YMB8GA1UdIwQYMBaAFDrWuuI5jajP
LQYnK/PQcD5uN03YMA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDRwAwRAIg
YSy7653vM0GNoxfm+gsFjy6mT9ao1VQtiomfQPHn5PUCIC5814t3SIq7chWd7gB0
jhbKKvEo2VyEXV2CVzVrdNGo
-----END CERTIFICATE-----
";
    pub(crate) const CA_B: &str = "-----BEGIN CERTIFICATE-----
MIIBkDCCATWgAwIBAgIUDwu0NVS0eEpEbg7szsTCNp0MA8gwCgYIKoZIzj0EAwIw
HDEaMBgGA1UEAwwRU2Nhbm9weSBUZXN0IENBIGIwIBcNMjYwOTI4MjEwNjA0WhgP
MjEyNjA5MDQyMTA2MDRaMBwxGjAYBgNVBAMMEVNjYW5vcHkgVGVzdCBDQSBiMFkw
EwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEZulS9nG3XLOhuUazgOywEdr9ETfuuKNg
WZDMMO3qfWgpdK1lbQpgoBkqQIQT3JE0alBGGZbWH5FuThlgGedqw6NTMFEwHQYD
VR0OBBYEFFm52DYbg0bM1aCD85hEMPrW2kbqMB8GA1UdIwQYMBaAFFm52DYbg0bM
1aCD85hEMPrW2kbqMA8GA1UdEwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDSQAwRgIh
AOnxnoWex0mLqM9xYpe6tBEQ5fvXWSd0j9xeuADx4pFkAiEAtCgwG4o+Vx0DH3pR
V2zF7MPMNdHDZSZ6SAJfCuoVZSs=
-----END CERTIFICATE-----
";
    const LEAF_CERT: &str = "-----BEGIN CERTIFICATE-----
MIIBwTCCAWegAwIBAgIURzCMoUtXS1kOIE7H379JjpWBLtEwCgYIKoZIzj0EAwIw
HDEaMBgGA1UEAwwRU2Nhbm9weSBUZXN0IENBIGEwIBcNMjYwOTI4MjEwNjE1WhgP
MjEyNjA5MDQyMTA2MTVaMBQxEjAQBgNVBAMMCWxvY2FsaG9zdDBZMBMGByqGSM49
AgEGCCqGSM49AwEHA0IABGzxtV5AgEEqBmYC6mVbzwxaCuLuZnuCa73lh3u5nG0l
tBctH0wwKtWxmtijlMg6V4eDNGlhbYO/ggMSJLjK3jajgYwwgYkwGgYDVR0RBBMw
EYIJbG9jYWxob3N0hwR/AAABMAkGA1UdEwQCMAAwCwYDVR0PBAQDAgeAMBMGA1Ud
JQQMMAoGCCsGAQUFBwMBMB0GA1UdDgQWBBSlzHiqWim5vjPEZaHwHt4NZGytHzAf
BgNVHSMEGDAWgBQ61rriOY2ozy0GJyvz0HA+bjdN2DAKBggqhkjOPQQDAgNIADBF
AiEA/u+shzDpsOcBbfXaA9Zi7rNikW/xM+ZDMsveSeGCRD0CIAh9VmZ5+I0Nx9KF
tRdpymIRp94Yk+Ak3wqAdR4Vb4H8
-----END CERTIFICATE-----
";
    pub(crate) const LEAF_KEY: &str = "-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgbwuF9+C2N9sV2fLc
KIT2PrDXn+7Ktl07trupEAevHvOhRANCAARs8bVeQIBBKgZmAuplW88MWgri7mZ7
gmu95Yd7uZxtJbQXLR9MMCrVsZrYo5TIOleHgzRpYW2Dv4IDEiS4yt42
-----END PRIVATE KEY-----
";

    /// Serve HTTPS on 127.0.0.1 with a leaf signed by CA A, answering every request with
    /// `200 ok`. Returns the port; reach it as `https://localhost:<port>/`.
    pub(crate) async fn serve_test_tls() -> u16 {
        let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from_pem_slice(LEAF_CERT.as_bytes()).unwrap()],
            PrivateKeyDer::from_pem_slice(LEAF_KEY.as_bytes()).unwrap(),
        )
        .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            while let Ok((stream, _)) = listener.accept().await {
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    let Ok(mut tls) = acceptor.accept(stream).await else {
                        return;
                    };
                    let mut buf = [0u8; 1024];
                    let _ = tls.read(&mut buf).await;
                    let _ = tls
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
                        )
                        .await;
                    let _ = tls.shutdown().await;
                });
            }
        });
        port
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{CA_A, CA_B, LEAF_KEY, serve_test_tls};
    use super::*;
    use std::io::Write;

    fn write_bundle(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        file
    }

    #[test]
    fn loads_every_certificate_in_a_bundle() {
        let file = write_bundle(&format!("{CA_A}\n{CA_B}"));
        let bundle = TrustedCaBundle::load(file.path()).unwrap();
        assert_eq!(bundle.len(), 2);
        assert_eq!(bundle.path(), file.path());
    }

    #[test]
    fn ignores_non_certificate_sections() {
        // Operators often concatenate a key or comments into the same file.
        let file = write_bundle(&format!("# private CA\n{LEAF_KEY}{CA_A}"));
        assert_eq!(TrustedCaBundle::load(file.path()).unwrap().len(), 1);
    }

    #[test]
    fn rejects_unusable_bundles_naming_the_path() {
        let corrupt = CA_A.replace("MIIBjjCC", "!!!!");
        let not_a_cert =
            "-----BEGIN CERTIFICATE-----\naGVsbG8gd29ybGQ=\n-----END CERTIFICATE-----\n";
        for contents in [
            "",
            "   \n\n",
            "this is not a certificate",
            LEAF_KEY,
            corrupt.as_str(),
            not_a_cert,
        ] {
            let file = write_bundle(contents);
            let err = TrustedCaBundle::load(file.path()).unwrap_err();
            assert!(
                format!("{err:#}").contains(&file.path().display().to_string()),
                "error for {contents:?} should name the path: {err:#}"
            );
        }
    }

    #[test]
    fn missing_file_error_names_the_path() {
        let path = Path::new("/nonexistent/scanopy-ca.pem");
        let err = TrustedCaBundle::load(path).unwrap_err();
        assert!(format!("{err:#}").contains("/nonexistent/scanopy-ca.pem"));
    }

    #[test]
    fn builds_smtp_tls_parameters() {
        let file = write_bundle(CA_A);
        let bundle = TrustedCaBundle::load(file.path()).unwrap();
        bundle
            .smtp_tls_parameters("mail.internal.example".to_string())
            .unwrap();
    }

    /// A reqwest client reaches a server whose leaf is signed by CA A only when the bundle
    /// is applied.
    #[tokio::test]
    async fn reqwest_trusts_a_private_ca_only_with_the_bundle() {
        let port = serve_test_tls().await;
        let url = format!("https://localhost:{port}/");

        let file = write_bundle(CA_A);
        let bundle = TrustedCaBundle::load(file.path()).unwrap();
        let trusting = TrustedCaBundle::apply(Some(&bundle), reqwest::Client::builder())
            .build()
            .unwrap();
        let body = trusting
            .get(&url)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert_eq!(body, "ok");

        let default = TrustedCaBundle::apply(None, reqwest::Client::builder())
            .build()
            .unwrap();
        assert!(default.get(&url).send().await.is_err());
    }

    /// An untrusted certificate is recognised through reqwest's error chain; a refused
    /// connection is not mistaken for one.
    #[tokio::test]
    async fn detects_untrusted_certificates_and_nothing_else() {
        let port = serve_test_tls().await;
        let client = reqwest::Client::new();

        let untrusted = client
            .get(format!("https://localhost:{port}/"))
            .send()
            .await
            .unwrap_err();
        assert!(is_untrusted_certificate(&untrusted));

        let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let closed_port = closed.local_addr().unwrap().port();
        drop(closed);
        let refused = client
            .get(format!("https://localhost:{closed_port}/"))
            .send()
            .await
            .unwrap_err();
        assert!(!is_untrusted_certificate(&refused));
    }
}
