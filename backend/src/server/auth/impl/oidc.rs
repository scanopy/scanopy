use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use openidconnect::{
    AuthenticationFlow, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
    core::{CoreClient, CoreProviderMetadata, CoreResponseType},
    reqwest::Client as ReqwestClient,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::server::config::DeploymentType;
use crate::server::shared::trusted_ca::is_untrusted_certificate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcPendingAuth {
    pub pkce_verifier: String,
    pub nonce: String,
    pub csrf_token: String,
    pub flow: OidcFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcRegisterParams<'a> {
    pub terms_accepted_at: Option<DateTime<Utc>>,
    pub deployment_type: DeploymentType,
    pub billing_enabled: bool,
    pub provider_slug: &'a str,
    pub code: &'a str,
    pub marketing_opt_in: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OidcFlow {
    Login,
    Register,
    Link,
}

#[derive(Debug, Clone)]
pub struct OidcUserInfo {
    pub subject: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OidcProviderConfig {
    pub name: String,
    pub slug: String,
    pub logo: Option<String>,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub struct OidcProviderMetadata {
    /// Display name of the identity provider, shown on the login button.
    pub name: String,
    /// URL-safe identifier used in the provider's login and link endpoints.
    pub slug: String,
    /// Logo to show on the login button, when the provider has one configured.
    pub logo: Option<String>,
}

impl OidcProviderConfig {
    pub fn to_metadata(&self) -> OidcProviderMetadata {
        OidcProviderMetadata {
            name: self.name.clone(),
            slug: self.slug.clone(),
            logo: self.logo.clone(),
        }
    }
}

/// Individual OIDC provider - just handles protocol operations
pub struct OidcProvider {
    pub slug: String,
    pub name: String,
    pub logo: Option<String>,
    issuer_url: String,
    client_id: String,
    client_secret: String,
    redirect_url: String,
    /// Shared client built by `OidcService::new`, carrying any extra trusted CA roots.
    http_client: ReqwestClient,
}

impl From<&OidcProvider> for OidcProviderMetadata {
    fn from(provider: &OidcProvider) -> Self {
        Self {
            name: provider.name.clone(),
            slug: provider.slug.clone(),
            logo: provider.logo.clone(),
        }
    }
}

impl OidcProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        slug: String,
        name: String,
        logo: Option<String>,
        issuer_url: String,
        client_id: String,
        client_secret: String,
        redirect_url: String,
        http_client: ReqwestClient,
    ) -> Self {
        Self {
            slug,
            name,
            logo,
            issuer_url,
            client_id,
            client_secret,
            redirect_url,
            http_client,
        }
    }

    /// Fetch the issuer's discovery document. openidconnect's error shows only "Request
    /// failed", so the full cause is logged here, and an untrusted certificate is named as such
    /// rather than left to read like an unreachable host.
    async fn discover(&self) -> Result<CoreProviderMetadata> {
        CoreProviderMetadata::discover_async(
            IssuerUrl::new(self.issuer_url.clone())?,
            &self.http_client,
        )
        .await
        .map_err(|e| {
            let cause =
                std::iter::successors(Some(&e as &(dyn std::error::Error + 'static)), |e| {
                    e.source()
                })
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(": ");
            tracing::warn!(
                provider = %self.slug,
                issuer = %self.issuer_url,
                error = %cause,
                "OIDC discovery failed"
            );
            if is_untrusted_certificate(&e) {
                anyhow!(
                    "the issuer's TLS certificate is signed by a CA this server doesn't trust. \
                     If it's a private CA, set SCANOPY_TRUSTED_CA_BUNDLE to its PEM file"
                )
            } else {
                anyhow!("{e}. Check that the issuer URL is reachable from the server")
            }
        })
    }

    /// Generate authorization URL for user to visit
    pub async fn authorize_url(&self, flow: OidcFlow) -> Result<(String, OidcPendingAuth)> {
        let provider_metadata = self.discover().await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::new(self.redirect_url.clone())?);

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let (auth_url, csrf_token, nonce) = client
            .authorize_url(
                AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();

        let pending_auth = OidcPendingAuth {
            pkce_verifier: pkce_verifier.secret().clone(),
            nonce: nonce.secret().clone(),
            csrf_token: csrf_token.secret().clone(),
            flow,
        };

        Ok((auth_url.to_string(), pending_auth))
    }

    /// Exchange authorization code for user info
    pub async fn exchange_code(
        &self,
        code: &str,
        pending_auth: &OidcPendingAuth,
    ) -> Result<OidcUserInfo> {
        let provider_metadata = self.discover().await?;

        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
        )
        .set_redirect_uri(RedirectUrl::new(self.redirect_url.clone())?);

        let pkce_verifier = PkceCodeVerifier::new(pending_auth.pkce_verifier.clone());
        let nonce = Nonce::new(pending_auth.nonce.clone());

        let token_response = client
            .exchange_code(AuthorizationCode::new(code.to_string()))?
            .set_pkce_verifier(pkce_verifier)
            .request_async(&self.http_client)
            .await?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow::anyhow!("No ID token in response"))?;

        let claims = id_token.claims(&client.id_token_verifier(), &nonce)?;

        Ok(OidcUserInfo {
            subject: claims.subject().to_string(),
            email: claims.email().map(|e| e.to_string()),
            name: claims
                .name()
                .and_then(|n| n.get(None).map(|s| s.to_string())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::shared::trusted_ca::test_support::serve_test_tls;

    /// An issuer behind a certificate the server doesn't trust must say so, not read like an
    /// unreachable host. openidconnect hides the cause in its Display, so this checks that it
    /// is still reachable through the error's source chain.
    #[tokio::test]
    async fn an_untrusted_issuer_certificate_is_named_in_the_error() {
        let port = serve_test_tls().await;
        let provider = OidcProvider::new(
            "private".to_string(),
            "Private".to_string(),
            None,
            format!("https://localhost:{port}"),
            "client".to_string(),
            "secret".to_string(),
            "https://scanopy.example/api/auth/oidc/private/callback".to_string(),
            ReqwestClient::new(),
        );

        let err = provider.authorize_url(OidcFlow::Login).await.unwrap_err();
        assert!(
            err.to_string().contains("SCANOPY_TRUSTED_CA_BUNDLE"),
            "{err}"
        );
    }
}
