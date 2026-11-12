use anyhow::{Result, anyhow};
use email_address::EmailAddress;
use std::{collections::HashMap, net::IpAddr, str::FromStr, sync::Arc};
use uuid::Uuid;

use crate::server::{
    auth::{
        disposable_email::is_disposable_email,
        r#impl::{
            base::{LoginRegisterParams, ProvisionUserParams},
            oidc::{
                OidcPendingAuth, OidcProvider, OidcProviderConfig, OidcProviderMetadata,
                OidcRegisterParams,
            },
        },
        middleware::auth::AuthenticatedEntity,
        service::AuthService,
    },
    config::DeploymentType,
    shared::{
        events::{
            bus::EventBus,
            traits::{AuthScope, Event},
            types::{AuthMethod, AuthOperation, UpdateReason},
        },
        services::traits::CrudService,
        storage::filter::StorableFilter,
        trusted_ca::TrustedCaBundle,
    },
    users::{r#impl::base::User, service::UserService},
};

/// Result of OIDC register — distinguishes new registration from auto-login of existing user
pub enum OidcRegisterResult {
    /// Brand new user was created
    NewUser(User),
    /// Existing user was found and logged in (OIDC subject matched)
    ExistingUser(User),
    /// Email already belongs to an existing account (no auto-link)
    EmailAlreadyExists,
}

/// A login refused because no account matches the provider's user. `login` has already
/// published `LoginFailed` with this text, so the handler doesn't log it again.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct OidcLoginRefused(pub String);

pub struct OidcService {
    providers: HashMap<String, Arc<OidcProvider>>,
    auth_service: Arc<AuthService>,
    user_service: Arc<UserService>,
    event_bus: Arc<EventBus>,
}

impl OidcService {
    pub fn new(
        configs: Vec<OidcProviderConfig>,
        public_url: &str,
        auth_service: Arc<AuthService>,
        user_service: Arc<UserService>,
        event_bus: Arc<EventBus>,
        trusted_ca: Option<&TrustedCaBundle>,
    ) -> Result<Self> {
        // One client for every provider's discovery, JWKS and token calls, so the extra CA
        // roots are added once rather than rebuilt per request.
        let http_client = TrustedCaBundle::apply(
            trusted_ca,
            reqwest::Client::builder().redirect(reqwest::redirect::Policy::limited(10)),
        )
        .build()?;

        let mut providers = HashMap::new();

        for config in configs {
            // Build provider-specific callback URL
            let redirect_url = format!(
                "{}/api/auth/oidc/{}/callback",
                public_url.trim_end_matches('/'),
                config.slug
            );

            let provider = OidcProvider::new(
                config.slug.clone(),
                config.name.clone(),
                config.servable_logo(),
                config.issuer_url.clone(),
                config.client_id.clone(),
                config.client_secret.clone(),
                config.trusted_audiences.clone(),
                redirect_url,
                http_client.clone(),
            );

            providers.insert(config.slug.clone(), Arc::new(provider));
        }

        Ok(Self {
            providers,
            auth_service,
            user_service,
            event_bus,
        })
    }

    pub fn get_provider(&self, slug: &str) -> Option<&Arc<OidcProvider>> {
        self.providers.get(slug)
    }

    pub fn list_providers(&self) -> Vec<OidcProviderMetadata> {
        self.providers
            .values()
            .map(|provider| OidcProviderMetadata {
                name: provider.name.clone(),
                slug: provider.slug.clone(),
                logo: provider.logo.clone(),
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }

    /// Register new user via OIDC, or auto-login if account already exists
    pub async fn register(
        &self,
        pending_auth: OidcPendingAuth,
        params: LoginRegisterParams,
        oidc_register_params: OidcRegisterParams<'_>,
    ) -> Result<OidcRegisterResult> {
        let OidcRegisterParams {
            provider_slug,
            code,
            terms_accepted_at,
            deployment_type,
        } = oidc_register_params;

        let provider = self
            .get_provider(provider_slug)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_slug))?;

        let LoginRegisterParams {
            provision_org,
            permissions,
            ip,
            user_agent,
            site_assignments,
            invited_as,
        } = params;

        // Exchange code for user info using provider
        let user_info = provider.exchange_code(code, &pending_auth).await?;

        // If user already exists with this OIDC account, log them in
        if let Some(existing_user) = self
            .user_service
            .get_user_by_oidc(&user_info.subject)
            .await?
        {
            // The handler publishes `LoginSuccess` once the session is saved.
            return Ok(OidcRegisterResult::ExistingUser(existing_user));
        }

        // Parse the provider's email, or fall back to a placeholder address
        let provider_email = user_info
            .email
            .as_deref()
            .and_then(|e| EmailAddress::from_str(e).ok());
        let email = provider_email.clone().unwrap_or_else(|| {
            EmailAddress::new_unchecked(format!("user{}@example.com", &user_info.subject[..8]))
        });

        // A placeholder address can't receive a verification link and an OIDC user can't
        // change it, so an account the provider gave no address is identified by the
        // provider alone and starts verified. Otherwise the provider's claim, a matching
        // invite, or a server with no email transport decides.
        let email_verified = match &provider_email {
            Some(address) => self.auth_service.verified_at_sign_up(
                address,
                user_info.email_verified,
                invited_as.as_ref(),
            ),
            None => true,
        };

        if deployment_type == DeploymentType::Cloud && is_disposable_email(&email) {
            return Err(anyhow!(
                "Email address uses a disposable domain. Please register with a non-disposable email address."
            ));
        }

        // Check if email is already in use by another account
        let existing = self
            .user_service
            .get_all(StorableFilter::<User>::new_from_email(&email))
            .await?;
        if !existing.is_empty() {
            return Ok(OidcRegisterResult::EmailAlreadyExists);
        }

        // Register new user
        let mut user = self
            .auth_service
            .provision_user(ProvisionUserParams {
                email,
                password_hash: None,
                oidc_subject: Some(user_info.subject),
                oidc_provider: Some(provider.slug.clone()),
                provision_org,
                email_verified,
                permissions,
                site_assignments,
                terms_accepted_at,
                deployment_type,
                ip_address: ip,
                user_agent: user_agent.clone(),
            })
            .await?;

        // Same as password sign-up: a failure here is logged, and the user can request a new
        // link from the resend endpoint.
        let email_and_token = if email_verified {
            None
        } else {
            match self
                .auth_service
                .set_user_email_verification_params(&mut user)
                .await
            {
                Ok(params) => Some(params),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to set email verification params");
                    None
                }
            }
        };

        // Publish event
        let authentication: AuthenticatedEntity = user.clone().into();
        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::Register {
                    method: AuthMethod::Oidc((&**provider).into()),
                    email_and_token,
                },
                authentication,
            ))
            .await;

        Ok(OidcRegisterResult::NewUser(user))
    }

    /// Login existing user via OIDC (fails if account doesn't exist). Publishes `LoginFailed` when
    /// the provider exchange fails or no account matches. `LoginSuccess` is the caller's to
    /// publish, with [`Self::publish_login_success`], once the session is saved.
    pub async fn login(
        &self,
        provider_slug: &str,
        code: &str,
        pending_auth: OidcPendingAuth,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        let provider = self
            .get_provider(provider_slug)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_slug))?;

        // Exchange code for user info using provider
        let user_info = match provider.exchange_code(code, &pending_auth).await {
            Ok(user_info) => user_info,
            Err(e) => {
                self.publish_login_failed(provider, None, format!("{e:#}"), ip, user_agent)
                    .await;
                return Err(e);
            }
        };

        // Check if user exists with this OIDC account
        if let Some(user) = self
            .user_service
            .get_user_by_oidc(&user_info.subject)
            .await?
        {
            return Ok(user);
        }

        let email = user_info
            .email
            .as_deref()
            .and_then(|e| EmailAddress::from_str(e).ok());
        // Check if an account exists with the same email but no OIDC link
        let unlinked = match &email {
            Some(email) => !self
                .user_service
                .get_all(StorableFilter::<User>::new_from_email(email))
                .await?
                .is_empty(),
            None => false,
        };
        let refused = if unlinked {
            OidcLoginRefused(format!(
                "An account with this email exists but isn't linked to {}. Please sign in with email first, then link your {} account from settings.",
                provider.name, provider.name
            ))
        } else {
            OidcLoginRefused(format!(
                "No account found with this {} login. Please register first.",
                provider.name
            ))
        };
        self.publish_login_failed(provider, email, refused.0.clone(), ip, user_agent)
            .await;
        Err(refused.into())
    }

    /// `LoginSuccess` for an OIDC sign-in, published by the handler once the session holds the
    /// user: a sign-in refused afterwards (a demo host, a session that won't save) never reports
    /// one.
    pub async fn publish_login_success(
        &self,
        user: &User,
        provider_slug: &str,
        via_register_flow: bool,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        let provider = self
            .get_provider(provider_slug)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_slug))?;
        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::LoginSuccess {
                    method: AuthMethod::Oidc((&**provider).into()),
                    via_register_flow,
                },
                user.clone().into(),
            ))
            .await;
        Ok(())
    }

    async fn publish_login_failed(
        &self,
        provider: &OidcProvider,
        attempted_email: Option<EmailAddress>,
        reason: String,
        ip: IpAddr,
        user_agent: Option<String>,
    ) {
        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: None,
                    organization_id: None,
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::LoginFailed {
                    method: AuthMethod::Oidc(provider.into()),
                    attempted_email,
                    reason,
                },
                AuthenticatedEntity::Anonymous,
            ))
            .await
    }

    /// Link OIDC account to existing user
    pub async fn link_to_user(
        &self,
        provider_slug: &str,
        user_id: &Uuid,
        code: &str,
        pending_auth: OidcPendingAuth,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        let provider = self
            .get_provider(provider_slug)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_slug))?;

        // Exchange code for user info using provider
        let user_info = provider.exchange_code(code, &pending_auth).await?;

        // Check if this OIDC account is already linked to another user
        if let Some(existing_user) = self
            .user_service
            .get_user_by_oidc(&user_info.subject)
            .await?
        {
            if existing_user.id != *user_id {
                return Err(anyhow!(
                    "This {} account is already linked to another user",
                    provider.name
                ));
            }
            // Already linked to this user
            return Ok(existing_user);
        }

        // Get and update user
        let mut user = self
            .user_service
            .get_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Verify OIDC email matches Scanopy account email
        if let Some(ref oidc_email) = user_info.email
            && !oidc_email.eq_ignore_ascii_case(user.base.email.as_ref())
        {
            return Err(anyhow!(
                "The email from your {} account ({}) doesn't match your Scanopy account email. \
                 Please use an account with the same email address.",
                provider.name,
                oidc_email
            ));
        }

        // ERROR if user already has a different OIDC provider linked
        if let Some(existing_provider) = &user.base.oidc_provider
            && existing_provider != provider_slug
        {
            let existing_provider_name = self
                .get_provider(existing_provider)
                .map(|p| p.name.as_str())
                .unwrap_or(existing_provider);

            return Err(anyhow!(
                "You already have {} linked to your account. Please unlink it first before linking {}.",
                existing_provider_name,
                provider.name
            ));
        }

        user.base.oidc_provider = Some(provider.slug.clone());
        user.base.oidc_subject = Some(user_info.subject);
        user.base.oidc_linked_at = Some(crate::server::shared::clock::now());

        let authentication: AuthenticatedEntity = user.clone().into();

        let user = self
            .user_service
            .update(
                &mut user,
                authentication.clone(),
                UpdateReason::UserOidcLinked,
            )
            .await?;

        // Published once the link is stored.
        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::OidcLinked {
                    provider: (&**provider).into(),
                    email: user.base.email.clone(),
                },
                authentication,
            ))
            .await;

        Ok(user)
    }

    /// Unlink OIDC from user
    pub async fn unlink_from_user(
        &self,
        provider_slug: &str,
        user_id: &Uuid,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        let provider = self
            .get_provider(provider_slug)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_slug))?;

        // Get user
        let mut user = self
            .user_service
            .get_by_id(user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        // Require password before unlinking
        if user.base.password_hash.is_none() {
            return Err(anyhow::anyhow!(
                "Cannot unlink OIDC - no password set. Set a password first."
            ));
        }

        user.base.oidc_provider = None;
        user.base.oidc_subject = None;
        user.base.oidc_linked_at = None;
        user.updated_at = crate::server::shared::clock::now();

        let authentication: AuthenticatedEntity = user.clone().into();

        let user = self
            .user_service
            .update(
                &mut user,
                authentication.clone(),
                UpdateReason::UserOidcUnlinked,
            )
            .await?;

        // Published once the unlink is stored.
        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::OidcUnlinked {
                    provider: (&**provider).into(),
                    email: user.base.email.clone(),
                },
                authentication,
            ))
            .await;

        Ok(user)
    }
}
