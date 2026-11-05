use std::error::Error as StdError;

use reqwest::StatusCode;
use rustls::CertificateError;
use serde::{Deserialize, Serialize};

use crate::server::shared::trusted_ca::certificate_error;
use crate::server::shared::types::error_codes::ErrorCode;

/// Why this server has no working entitlement, as the public config endpoint
/// reports it. `/api/config` needs no login, so it carries this type alone:
/// addresses, URLs and raw error text stay in the server log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LicenseProblem {
    /// The license server's hostname did not resolve.
    Dns,
    /// The connection failed: a firewall, a route, or a refused port.
    Connect,
    /// The check-in got no answer in time.
    Timeout,
    /// The TLS certificate presented is not trusted, usually TLS interception.
    UntrustedCertificate,
    /// The TLS certificate is outside its validity dates, usually a wrong clock.
    CertificateDates,
    /// A proxy or firewall answered instead of the license server.
    Proxy,
    /// The license server answered with an error of its own.
    ServerError,
    /// The license server asked this server to slow down.
    RateLimited,
    /// The configured key does not decode or verify.
    KeyMalformed,
    /// The key was rotated or changed type, so this copy is retired.
    KeyRegenerated,
    /// The organization the key names no longer exists.
    OrganizationMissing,
    /// The organization moved to a plan with no license key.
    PlanHasNoKey,
    /// The key already runs on as many servers as the plan covers.
    InstanceLimit,
    /// The license server refused the key for another reason.
    KeyRefused,
}

impl LicenseProblem {
    /// Cause and next step for the server log.
    pub fn log_message(self) -> &'static str {
        match self {
            Self::Dns => {
                "the license server's hostname did not resolve. Check this server's DNS settings"
            }
            Self::Connect => {
                "could not connect to the license server. Allow outbound HTTPS to it, or set HTTPS_PROXY if this network requires a proxy"
            }
            Self::Timeout => {
                "the license server did not answer in time. Allow outbound HTTPS to it, or set HTTPS_PROXY if this network requires a proxy"
            }
            Self::UntrustedCertificate => {
                "the certificate presented is not trusted. If this network intercepts TLS, set SCANOPY_TRUSTED_CA_BUNDLE to your CA bundle"
            }
            Self::CertificateDates => {
                "the certificate presented is outside its validity dates. Check this server's clock"
            }
            Self::Proxy => {
                "a proxy or firewall answered instead of the license server. Allow this server through to the license server"
            }
            Self::ServerError => "the license server returned an error. This server keeps retrying",
            Self::RateLimited => {
                "the license server is rate limiting this address. This server keeps retrying"
            }
            Self::KeyMalformed => {
                "the license key is incomplete or altered. Copy it again from Settings > License in Scanopy Cloud"
            }
            Self::KeyRegenerated => {
                "this license key was replaced. Copy the current key from Settings > License in Scanopy Cloud"
            }
            Self::OrganizationMissing => {
                "the organization this key belongs to no longer exists. Contact licensing@scanopy.net"
            }
            Self::PlanHasNoKey => {
                "this organization's plan has no license key. Choose a self-hosted plan in Scanopy Cloud"
            }
            Self::InstanceLimit => {
                "this key already runs on as many servers as the plan covers. Delete a server you no longer run under Settings > License in Scanopy Cloud, then restart this one"
            }
            Self::KeyRefused => {
                "the license server refused this key. Copy it again from Settings > License in Scanopy Cloud"
            }
        }
    }

    /// Classify a request that got no HTTP response.
    pub fn from_transport(err: &reqwest::Error) -> Self {
        match certificate_error(err) {
            Some(
                CertificateError::Expired
                | CertificateError::ExpiredContext { .. }
                | CertificateError::NotValidYet
                | CertificateError::NotValidYetContext { .. },
            ) => return Self::CertificateDates,
            Some(_) => return Self::UntrustedCertificate,
            None => {}
        }
        if err.is_timeout() {
            return Self::Timeout;
        }
        // hyper reports these as plain messages with no typed error to match.
        let chain = error_chain(err);
        if chain.contains("tunnel") || chain.contains("proxy") {
            Self::Proxy
        } else if chain.contains("dns error") {
            Self::Dns
        } else {
            Self::Connect
        }
    }

    /// Classify a response other than a success or a refusal of the key.
    pub fn from_status(status: StatusCode) -> Self {
        match status {
            StatusCode::TOO_MANY_REQUESTS => Self::RateLimited,
            StatusCode::UNAUTHORIZED | StatusCode::PROXY_AUTHENTICATION_REQUIRED => Self::Proxy,
            _ => Self::ServerError,
        }
    }

    /// Classify a refusal by the error code the license server sent.
    pub fn from_refusal_code(code: Option<&str>) -> Self {
        let Some(code) = code else {
            return Self::KeyRefused;
        };
        if code == ErrorCode::LicenseKeyRegenerated.code() {
            Self::KeyRegenerated
        } else if code == ErrorCode::LicenseOrganizationMissing.code() {
            Self::OrganizationMissing
        } else if code == ErrorCode::LicensePlanHasNoKey.code() {
            Self::PlanHasNoKey
        } else if code
            == (ErrorCode::LicenseInstanceLimitReached {
                active: 0,
                limit: 0,
            })
            .code()
        {
            Self::InstanceLimit
        } else {
            Self::KeyRefused
        }
    }
}

/// `err` and its sources as one string. reqwest's own `Display` stops at
/// "error sending request", which hides what actually failed.
pub fn error_chain(err: &(dyn StdError + 'static)) -> String {
    let mut parts = vec![err.to_string()];
    let mut current = err.source();
    while let Some(e) = current {
        parts.push(e.to_string());
        current = e.source();
    }
    parts.join(": ")
}
