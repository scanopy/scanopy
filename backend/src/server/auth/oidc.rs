use anyhow::{Error, Result, anyhow};
use email_address::EmailAddress;
use std::{collections::HashMap, net::IpAddr, str::FromStr, sync::Arc};
use uuid::Uuid;

use crate::server::{
    auth::{
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
            types::{AuthMethod, AuthOperation},
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
                config.logo.clone(),
                config.issuer_url.clone(),
                config.client_id.clone(),
                config.client_secret.clone(),
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
            billing_enabled,
            terms_accepted_at,
            deployment_type,
            marketing_opt_in,
        } = oidc_register_params;

        let provider = self
            .get_provider(provider_slug)
            .ok_or_else(|| anyhow!("Provider '{}' not found", provider_slug))?;

        let LoginRegisterParams {
            provision_org,
            permissions,
            ip,
            user_agent,
            site_ids,
        } = params;

        // Exchange code for user info using provider
        let user_info = provider.exchange_code(code, &pending_auth).await?;

        // If user already exists with this OIDC account, log them in
        if let Some(existing_user) = self
            .user_service
            .get_user_by_oidc(&user_info.subject)
            .await?
        {
            let authentication: AuthenticatedEntity = existing_user.clone().into();
            self.event_bus
                .publish(Event::new(
                    AuthScope {
                        user_id: Some(existing_user.id),
                        organization_id: Some(existing_user.base.organization_id),
                        ip_address: ip,
                        user_agent,
                    },
                    AuthOperation::LoginSuccess {
                        method: AuthMethod::Oidc((&**provider).into()),
                        via_register_flow: true,
                    },
                    authentication,
                ))
                .await?;
            return Ok(OidcRegisterResult::ExistingUser(existing_user));
        }

        // Parse or create fallback email
        let fallback_email_str = format!("user{}@example.com", &user_info.subject[..8]);
        let email_str = user_info
            .email
            .clone()
            .unwrap_or_else(|| fallback_email_str.clone());

        let email = EmailAddress::from_str(&email_str).or_else(|_| {
            Ok::<EmailAddress, Error>(EmailAddress::new_unchecked(fallback_email_str))
        })?;

        if !mailchecker::is_valid(email.as_str()) && deployment_type == DeploymentType::Cloud {
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
        let user = self
            .auth_service
            .provision_user(ProvisionUserParams {
                email,
                password_hash: None,
                oidc_subject: Some(user_info.subject),
                oidc_provider: Some(provider.slug.clone()),
                provision_org,
                email_verified: true,
                permissions,
                site_ids,
                terms_accepted_at,
                billing_enabled,
            })
            .await?;

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
                    marketing_opt_in,
                    // No email verification sent for OIDC logins
                    email_and_token: None,
                },
                authentication,
            ))
            .await?;

        Ok(OidcRegisterResult::NewUser(user))
    }

    /// Login existing user via OIDC (fails if account doesn't exist)
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
        let user_info = provider.exchange_code(code, &pending_auth).await?;

        // Check if user exists with this OIDC account
        let user = match self
            .user_service
            .get_user_by_oidc(&user_info.subject)
            .await?
        {
            Some(user) => user,
            None => {
                // Check if an account exists with the same email but no OIDC link
                if let Some(email_str) = &user_info.email
                    && let Ok(email) = EmailAddress::from_str(email_str)
                {
                    let existing = self
                        .user_service
                        .get_all(StorableFilter::<User>::new_from_email(&email))
                        .await?;
                    if !existing.is_empty() {
                        return Err(anyhow!(
                            "An account with this email exists but isn't linked to {}. Please sign in with email first, then link your {} account from settings.",
                            provider.name,
                            provider.name
                        ));
                    }
                }
                return Err(anyhow!(
                    "No account found with this {} login. Please register first.",
                    provider.name
                ));
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
                AuthOperation::LoginSuccess {
                    method: AuthMethod::Oidc((&**provider).into()),
                    via_register_flow: false,
                },
                authentication,
            ))
            .await?;

        Ok(user)
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
        user.base.oidc_linked_at = Some(chrono::Utc::now());

        let authentication: AuthenticatedEntity = user.clone().into();

        // Publish event
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
                authentication.clone(),
            ))
            .await?;

        self.user_service.update(&mut user, authentication).await
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
        user.updated_at = chrono::Utc::now();

        let authentication: AuthenticatedEntity = user.clone().into();

        // Publish event
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
                authentication.clone(),
            ))
            .await?;

        self.user_service.update(&mut user, authentication).await
    }
}
