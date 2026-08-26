//! The gNMI operations the collector needs, behind a trait so parsing and mapping run in CI
//! against scripted devices with no network — the [`SnmpWalkTransport`] arrangement. Two
//! implementors: the tonic channel below in production, and canned-notification fakes under
//! `#[cfg(test)]` in `super::tests`.
//!
//! [`SnmpWalkTransport`]: crate::daemon::discovery::integration::snmp::queries::SnmpWalkTransport

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::service::Interceptor;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::{Request, Status};

use super::proto::gnmi::{
    CapabilityRequest, Encoding, Notification, Path, SubscribeRequest, Subscription,
    SubscriptionList, g_nmi_client::GNmiClient, subscribe_request, subscribe_response,
    subscription_list,
};
use crate::server::credentials::r#impl::mapping::{GnmiQueryCredential, ResolvableSecret};

/// TCP + TLS + HTTP/2 preface. Devices answer or they don't; there is no slow path here.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
/// Bound on one RPC end to end. A Subscribe ONCE over `/interfaces` on a chassis with a few
/// thousand rows streams for seconds, not minutes.
pub const RPC_TIMEOUT: Duration = Duration::from_secs(60);

/// What the collector asks of a device. One method per gNMI RPC it uses.
#[async_trait]
pub trait GnmiTransport: Send {
    /// `Capabilities`: the cheapest authenticated round trip, so what `probe` checks. A wrong
    /// password fails here with `UNAUTHENTICATED`, a non-gNMI gRPC listener with `UNIMPLEMENTED`.
    async fn capabilities(&mut self) -> Result<()>;

    /// `Subscribe` with `mode: ONCE` for the given paths, returning every notification the
    /// device sent before its `sync_response`. The one read the collector relies on: ArcOS
    /// rejects subtree `Get`s (and times out on unkeyed ones) but answers ONCE for anything.
    async fn subscribe_once(&mut self, paths: Vec<Path>) -> Result<Vec<Notification>>;
}

/// Why a dial did not produce a transport, in the two shapes `probe` reports differently.
#[derive(Debug)]
pub enum ConnectError {
    /// The credential asks for something this build cannot honour. Not the device's fault.
    Unsupported(String),
    /// The device did not accept a connection: refused, timed out, unreachable.
    Dial(String),
    /// TCP connected but the TLS handshake did not complete.
    Tls(String),
}

impl std::fmt::Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(m) | Self::Dial(m) | Self::Tls(m) => f.write_str(m),
        }
    }
}

/// Attaches the credential as `username`/`password` request metadata — the OpenConfig
/// convention, and what gnmic sends. There is no gRPC-level auth in gNMI itself.
#[derive(Clone)]
pub struct AuthMetadata {
    username: MetadataValue<Ascii>,
    password: MetadataValue<Ascii>,
}

impl Interceptor for AuthMetadata {
    fn call(&mut self, mut req: Request<()>) -> std::result::Result<Request<()>, Status> {
        req.metadata_mut().insert("username", self.username.clone());
        req.metadata_mut().insert("password", self.password.clone());
        Ok(req)
    }
}

type Client = GNmiClient<InterceptedService<Channel, AuthMetadata>>;

/// The production transport: one tonic channel to one device.
pub struct TonicTransport {
    client: Client,
    cancel: CancellationToken,
}

impl TonicTransport {
    pub async fn connect(
        ip: IpAddr,
        cred: &GnmiQueryCredential,
        cancel: CancellationToken,
    ) -> std::result::Result<Self, ConnectError> {
        let password = match &cred.password {
            ResolvableSecret::Value { value } => value.clone(),
            // resolve_file_paths runs before dispatch; a FilePath surviving to here is a bug
            // upstream of this integration, not something to silently read again.
            ResolvableSecret::FilePath { path } => {
                return Err(ConnectError::Unsupported(format!(
                    "gNMI password still unresolved (file path {path}); expected an inline value"
                )));
            }
        };
        let auth = AuthMetadata {
            username: MetadataValue::try_from(cred.username.as_str()).map_err(|_| {
                ConnectError::Unsupported("gNMI username is not valid ASCII metadata".into())
            })?,
            password: MetadataValue::try_from(password.as_str()).map_err(|_| {
                ConnectError::Unsupported("gNMI password is not valid ASCII metadata".into())
            })?,
        };

        // `SocketAddr` brackets an IPv6 literal, which a URI authority requires.
        let scheme = if cred.tls { "https" } else { "http" };
        let uri = format!("{scheme}://{}", SocketAddr::new(ip, cred.port));
        let mut endpoint = Endpoint::from_shared(uri)
            .map_err(|e| ConnectError::Unsupported(format!("bad gNMI endpoint: {e}")))?
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(RPC_TIMEOUT);
        if cred.skip_verify {
            // Not yet: tonic 0.14.2 (what the workspace resolves to) has no way to install a
            // custom certificate verifier; `tls_config_with_verifier` arrives in 0.14.6. The
            // field stays on the credential so the wire shape is settled; the error says what
            // the build cannot do rather than quietly verifying anyway, or not at all.
            return Err(ConnectError::Unsupported(
                "gNMI skip_verify is not supported by this build yet; use tls with a \
                 certificate the webpki roots can verify, or plaintext"
                    .into(),
            ));
        }
        if cred.tls {
            // Addressed by IP, so SNI carries the IP literal; verification is against the
            // webpki roots.
            endpoint = endpoint
                .tls_config(ClientTlsConfig::new().with_webpki_roots())
                .map_err(|e| ConnectError::Unsupported(format!("gNMI TLS configuration: {e}")))?;
        }

        let channel = endpoint.connect().await.map_err(|e| {
            // tonic's transport error is opaque, so the handshake is told apart from a plain
            // ECONNREFUSED by the wording of the error chain: a refused TCP connect on a TLS
            // credential is still "unreachable", not "TLS failed".
            let chain = format!(
                "{e}: {}",
                std::error::Error::source(&e)
                    .map(ToString::to_string)
                    .unwrap_or_default()
            );
            let lower = chain.to_lowercase();
            if cred.tls
                && ["tls", "certificate", "handshake"]
                    .iter()
                    .any(|k| lower.contains(k))
            {
                ConnectError::Tls(format!("gNMI TLS handshake failed: {chain}"))
            } else {
                ConnectError::Dial(format!("gNMI connect failed: {chain}"))
            }
        })?;
        Ok(Self {
            client: GNmiClient::with_interceptor(channel, auth),
            cancel,
        })
    }
}

#[async_trait]
impl GnmiTransport for TonicTransport {
    async fn capabilities(&mut self) -> Result<()> {
        let call = self.client.capabilities(CapabilityRequest::default());
        tokio::select! {
            _ = self.cancel.cancelled() => Err(anyhow!("Discovery cancelled")),
            r = tokio::time::timeout(CONNECT_TIMEOUT, call) => {
                r.map_err(|_| anyhow!("gNMI Capabilities timed out"))?
                    .map_err(|status| anyhow!("gNMI Capabilities failed: {status}"))?;
                Ok(())
            }
        }
    }

    async fn subscribe_once(&mut self, paths: Vec<Path>) -> Result<Vec<Notification>> {
        let request = SubscribeRequest {
            request: Some(subscribe_request::Request::Subscribe(SubscriptionList {
                subscription: paths
                    .into_iter()
                    .map(|path| Subscription {
                        path: Some(path),
                        ..Default::default()
                    })
                    .collect(),
                mode: subscription_list::Mode::Once as i32,
                // PROTO, not JSON_IETF: ArcOS rejects the JSON encodings on Subscribe while
                // advertising them in Capabilities, and per-leaf typed values parse the same
                // whichever encoding a device prefers. `absorb` still accepts JSON blobs from
                // devices that answer with them regardless.
                encoding: Encoding::Proto as i32,
                ..Default::default()
            })),
            ..Default::default()
        };
        let mut stream = self
            .client
            .subscribe(tokio_stream::once(request))
            .await
            .map_err(|status| anyhow!("gNMI Subscribe failed: {status}"))?
            .into_inner();

        let mut notifications = Vec::new();
        loop {
            let next = tokio::select! {
                _ = self.cancel.cancelled() => return Err(anyhow!("Discovery cancelled")),
                r = tokio::time::timeout(RPC_TIMEOUT, stream.message()) => r,
            };
            let msg = match next {
                Ok(Ok(Some(m))) => m,
                // A device that closes without `sync_response` has said all it will; what
                // arrived is what there is.
                Ok(Ok(None)) => break,
                Ok(Err(status)) => {
                    return Err(anyhow!("gNMI Subscribe stream error: {status}"));
                }
                Err(_) => return Err(anyhow!("gNMI Subscribe stream timed out")),
            };
            // The deprecated `Error` arm is still what some devices send in place of a status.
            #[allow(deprecated)]
            match msg.response {
                Some(subscribe_response::Response::Update(n)) => notifications.push(n),
                Some(subscribe_response::Response::SyncResponse(_)) => break,
                Some(subscribe_response::Response::Error(e)) => {
                    return Err(anyhow!("gNMI Subscribe error from device: {}", e.message));
                }
                None => {}
            }
        }
        Ok(notifications)
    }
}
