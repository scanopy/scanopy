use crate::server::auth::r#impl::base::ProvisionOrg;
use crate::server::billing::types::base::BillingPlan;
use crate::server::shared::events::types::{
    EmailAndToken, OnboardingOperation, OnboardingOperationDiscriminants,
};
use crate::server::shared::types::api::ApiError;
use crate::server::shared::types::error_codes::ErrorCode;
use crate::server::{
    auth::{
        r#impl::{
            api::{LoginRequest, RegisterRequest},
            base::{LoginRegisterParams, PendingSetup, ProvisionUserParams},
        },
        middleware::auth::AuthenticatedEntity,
    },
    organizations::{
        r#impl::base::{Organization, OrganizationBase},
        service::OrganizationService,
    },
    shared::{
        events::{
            bus::EventBus,
            traits::{AuthScope, Event, OrgScope},
            types::{AuthMethod, AuthOperation},
        },
        services::traits::CrudService,
        storage::{filter::StorableFilter, traits::Storable},
    },
    users::{
        r#impl::{
            base::{User, UserBase},
            permissions::UserOrgPermissions,
        },
        service::UserService,
    },
};
use anyhow::{Result, anyhow};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use axum::http::StatusCode;
use base64ct::{Base64UrlUnpadded, Encoding};
use chrono::{Duration, Utc};
use email_address::EmailAddress;
use rand::RngCore;
use std::{collections::HashMap, net::IpAddr, sync::Arc, time::Instant};
use tokio::sync::RwLock;
use uuid::Uuid;
use validator::Validate;

pub struct AuthService {
    pub user_service: Arc<UserService>,
    organization_service: Arc<OrganizationService>,
    has_email_service: bool,
    login_attempts: Arc<RwLock<HashMap<EmailAddress, (u32, Instant)>>>,
    /// Rate limiting for verification email resend (not token storage - tokens stored in DB)
    verification_resend_cooldown: Arc<RwLock<HashMap<EmailAddress, Instant>>>,
    event_bus: Arc<EventBus>,
    /// Plan assigned to a new org on a self-hosted deployment
    /// (`plans::self_hosted_plan`). On cloud this is unused (new orgs get
    /// `plan = None` until Stripe checkout). Its `included_orgs` also drives
    /// the self-hosted org-creation cap.
    default_self_hosted_plan: BillingPlan,
}

impl AuthService {
    const MAX_LOGIN_ATTEMPTS: u32 = 5;
    const LOCKOUT_DURATION_SECS: u64 = 15 * 60; // 15 minutes
    const VERIFICATION_TOKEN_EXPIRY_HOURS: i64 = 24;
    const PASSWORD_RESET_TOKEN_EXPIRY_HOURS: i64 = 1;
    const RESEND_COOLDOWN_SECS: u64 = 60;

    /// Generate a 256-bit CSPRNG token encoded as base64url (no padding).
    fn generate_secure_token() -> String {
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        Base64UrlUnpadded::encode_string(&bytes)
    }

    pub fn new(
        user_service: Arc<UserService>,
        organization_service: Arc<OrganizationService>,
        has_email_service: bool,
        event_bus: Arc<EventBus>,
        default_self_hosted_plan: BillingPlan,
    ) -> Self {
        Self {
            user_service,
            organization_service,
            has_email_service,
            login_attempts: Arc::new(RwLock::new(HashMap::new())),
            verification_resend_cooldown: Arc::new(RwLock::new(HashMap::new())),
            event_bus,
            default_self_hosted_plan,
        }
    }

    // /// Send OIDC linked notification email (non-blocking)
    // pub async fn send_oidc_linked_notification(&self, email: EmailAddress, provider_name: &str) {
    //     if let Some(email_service) = &self.email_service
    //         && let Err(e) = email_service
    //             .send_oidc_linked_email(email, provider_name)
    //             .await
    //     {
    //         tracing::warn!(error = %e, "Failed to send OIDC linked notification email");
    //     }
    // }

    // /// Send OIDC unlinked notification email (non-blocking)
    // pub async fn send_oidc_unlinked_notification(&self, email: EmailAddress, provider_name: &str) {
    //     if let Some(email_service) = &self.email_service
    //         && let Err(e) = email_service
    //             .send_oidc_unlinked_email(email, provider_name)
    //             .await
    //     {
    //         tracing::warn!(error = %e, "Failed to send OIDC unlinked notification email");
    //     }
    // }

    /// Register a new user with password
    pub async fn register(
        &self,
        request: RegisterRequest,
        params: LoginRegisterParams,
        billing_enabled: bool,
    ) -> Result<User> {
        let LoginRegisterParams {
            provision_org,
            permissions,
            ip,
            user_agent,
            network_ids,
        } = params;

        request
            .validate()
            .map_err(|e| anyhow!("Validation failed: {}", e))?;

        // Check if email already taken
        let all_users = self
            .user_service
            .get_all(StorableFilter::<User>::new_from_email(&request.email))
            .await?;

        if !all_users.is_empty() {
            return Err(anyhow!("Email address already taken"));
        }

        let terms_accepted_at = if request.terms_accepted {
            Some(Utc::now())
        } else {
            None
        };

        // Auto-verify when:
        // - invited user joining existing org (invite link proves email ownership), or
        // - no email service is configured (self-hosted without SMTP/Brevo —
        //   no other way for the user to receive a verification token).
        // Otherwise leave `email_verified = false` and emit a verification email.
        let auto_verify = provision_org.is_existing() || !self.has_email_service;

        // Provision user with password
        let mut user = self
            .provision_user(ProvisionUserParams {
                email: request.email,
                password_hash: Some(hash_password(&request.password)?),
                oidc_subject: None,
                oidc_provider: None,
                email_verified: auto_verify,
                provision_org,
                permissions,
                network_ids,
                terms_accepted_at,
                billing_enabled,
            })
            .await?;

        // Generate the verification token (and persist it on the user record)
        // before emitting the event so the email subscriber receives the token
        // in the payload. If token generation fails, log and continue — the
        // user can request a new verification email via the resend endpoint.
        let email_and_token = if auto_verify {
            None
        } else {
            match self.set_user_email_verification_params(&mut user).await {
                Ok(params) => Some(params),
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to set email verification params");
                    None
                }
            }
        };

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
                    method: AuthMethod::Password,
                    marketing_opt_in: request.marketing_opt_in,
                    email_and_token,
                },
                authentication,
            ))
            .await?;

        Ok(user)
    }

    /// Core user provisioning logic - handles both password and OIDC registration
    /// If pending_setup is provided, uses setup.org_name and marks OnboardingModalCompleted
    /// If billing_enabled is false (self-hosted), sets default billing plan
    pub async fn provision_user(&self, params: ProvisionUserParams) -> Result<User> {
        let ProvisionUserParams {
            email,
            password_hash,
            oidc_subject,
            oidc_provider,
            provision_org,
            email_verified,
            permissions,
            network_ids,
            terms_accepted_at,
            billing_enabled,
        } = params;

        // Plan at org creation time — surfaced on the OrgCreated onboarding
        // event for downstream Brevo/PostHog identify.
        let mut new_org_plan: Option<BillingPlan> = None;

        let (organization_id, permissions) = match provision_org.clone() {
            ProvisionOrg::Existing(org_id) => {
                (org_id, permissions.unwrap_or(UserOrgPermissions::Viewer))
            }
            ProvisionOrg::New(PendingSetup {
                use_case, org_name, ..
            }) => {
                // Self-hosted org count is capped by the plan's
                // `included_orgs`. `None` = unlimited = no cap, which is what
                // the self-hosted plan carries, so this gate is inert unless
                // that plan is edited. Cloud (billing_enabled) stays
                // multi-tenant regardless.
                // Gates creation only — instances already above the cap keep
                // their orgs. Invited users take the `Existing` arm and are
                // unaffected. `billing_enabled == false` ⇔ self-hosted.
                if !billing_enabled
                    && let Some(max_orgs) = self.default_self_hosted_plan.config().included_orgs
                {
                    let org_count = self
                        .organization_service
                        .get_all(StorableFilter::<Organization>::new())
                        .await?
                        .len();
                    if org_count >= max_orgs as usize {
                        return Err(ApiError::coded(
                            StatusCode::FORBIDDEN,
                            ErrorCode::AuthOrgLimitReached,
                        )
                        .into());
                    }
                }

                let onboarding = vec![OnboardingOperationDiscriminants::OnboardingModalCompleted];

                // Cloud: no plan until user selects one via billing modal → Stripe checkout → webhook
                // Self-hosted: assign the self-hosted plan and emit a
                // CheckoutCompleted event below so the ledger reflects it immediately.
                let self_hosted_plan = if billing_enabled {
                    None
                } else {
                    Some(self.default_self_hosted_plan)
                };
                new_org_plan = self_hosted_plan;

                // Create new organization for this user
                let organization = self
                    .organization_service
                    .create(
                        Organization::new(OrganizationBase {
                            stripe_customer_id: None,
                            name: org_name,
                            plan: self_hosted_plan,
                            plan_status: None,
                            onboarding,
                            has_payment_method: false,
                            trial_end_date: None,
                            last_paused_at: None,
                            trial_extended_used: false,
                            last_downgrade_at: None,
                            last_downgrade_from_plan: None,
                            last_discount_at: None,
                            discount_save_offer_percent_off: None,
                            discount_save_offer_active_until: None,
                            next_renewal_at: None,
                            brevo_company_id: None,
                            notifications: Default::default(),
                            use_case,
                        }),
                        AuthenticatedEntity::System,
                    )
                    .await?;

                (organization.id, UserOrgPermissions::Owner)
            }
        };

        // Create user based on auth method
        let user = if let Some(hash) = password_hash {
            Ok(self
                .user_service
                .create(
                    User::new(UserBase::new_password(
                        email,
                        email_verified,
                        hash,
                        organization_id,
                        permissions,
                        network_ids,
                        terms_accepted_at,
                    )),
                    AuthenticatedEntity::System,
                )
                .await?)
        } else if let Some(oidc_subject) = oidc_subject {
            Ok(self
                .user_service
                .create(
                    User::new(UserBase::new_oidc(
                        email,
                        oidc_subject,
                        oidc_provider,
                        organization_id,
                        permissions,
                        network_ids,
                        terms_accepted_at,
                    )),
                    AuthenticatedEntity::System,
                )
                .await?)
        } else {
            Err(anyhow!("Must provide either password or OIDC credentials"))
        }?;

        if let ProvisionOrg::New(PendingSetup {
            org_name, use_case, ..
        }) = provision_org
        {
            let authentication: AuthenticatedEntity = user.clone().into();
            let plan = new_org_plan.unwrap_or_default();

            self.event_bus
                .publish(Event::new(
                    OrgScope { organization_id },
                    OnboardingOperation::OrgCreated {
                        org_name,
                        plan,
                        use_case,
                    },
                    authentication,
                ))
                .await?;
        }

        Ok(user)
    }

    /// Login with username and password
    pub async fn login(
        &self,
        request: LoginRequest,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        request
            .validate()
            .map_err(|e| anyhow!("Validation failed: {}", e))?;

        // Check if account is locked due to too many failed attempts
        self.check_login_lockout(&request.email).await?;

        // Attempt login
        let result = self.try_login(&request).await;

        // Update login attempts based on result
        match result {
            Ok(user) => {
                // Success - clear attempts
                self.login_attempts.write().await.remove(&request.email);

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
                            method: AuthMethod::Password,
                            via_register_flow: false,
                        },
                        authentication,
                    ))
                    .await?;

                Ok(user)
            }
            Err(e) => {
                // Failure - increment attempts

                self.event_bus
                    .publish(Event::new(
                        AuthScope {
                            user_id: None,
                            organization_id: None,
                            ip_address: ip,
                            user_agent,
                        },
                        AuthOperation::LoginFailed {
                            method: AuthMethod::Password,
                            attempted_email: request.email.clone(),
                        },
                        AuthenticatedEntity::Anonymous,
                    ))
                    .await?;

                let mut attempts = self.login_attempts.write().await;
                let entry = attempts
                    .entry(request.email.clone())
                    .or_insert((0, Instant::now()));
                entry.0 += 1;
                entry.1 = Instant::now();
                Err(e)
            }
        }
    }

    /// Check if user is locked out due to too many login attempts
    async fn check_login_lockout(&self, email: &EmailAddress) -> Result<()> {
        let attempts = self.login_attempts.read().await;
        if let Some((count, last_attempt)) = attempts.get(email)
            && *count >= Self::MAX_LOGIN_ATTEMPTS
        {
            let elapsed = last_attempt.elapsed().as_secs();
            if elapsed < Self::LOCKOUT_DURATION_SECS {
                let remaining = (Self::LOCKOUT_DURATION_SECS - elapsed) / 60;
                return Err(anyhow!(
                    "Too many failed login attempts. Try again in {} minutes.",
                    remaining + 1
                ));
            }
        }
        Ok(())
    }

    /// Attempt login without rate limiting
    async fn try_login(&self, request: &LoginRequest) -> Result<User> {
        // Fixed dummy hash used to equalize work on the "no such user" / "user
        // without a password" paths. Verifying against this takes the same
        // Argon2 time as a real account, so response latency doesn't reveal
        // which emails are registered (timing-based user enumeration).
        static DUMMY_PASSWORD_HASH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
            hash_password("timing-equalizer-not-a-real-password").unwrap_or_default()
        });

        let user = self
            .user_service
            .get_unique(StorableFilter::<User>::new_from_email(&request.email))
            .await?
            .at_most_one()?;

        // Always run a password verification, against the real hash when
        // present and the dummy hash otherwise, before deciding success.
        let hash_to_check = user
            .as_ref()
            .and_then(|u| u.base.password_hash.clone())
            .unwrap_or_else(|| DUMMY_PASSWORD_HASH.clone());

        let verified = verify_password(&request.password, &hash_to_check).is_ok();

        match user {
            Some(user) if verified => Ok(user),
            _ => Err(anyhow!("Invalid email or password")),
        }
    }

    pub async fn update_password(
        &self,
        user_id: Uuid,
        current_password: Option<String>,
        new_password: String,
        ip: IpAddr,
        user_agent: Option<String>,
        authentication: AuthenticatedEntity,
    ) -> Result<User> {
        let mut user = self
            .user_service
            .get_by_id(&user_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("User not found".to_string()))?;

        let had_password = user.base.password_hash.is_some();

        // If user already has a password, verify the current one
        if had_password {
            let current = current_password
                .ok_or_else(|| anyhow!("Current password is required to change your password"))?;
            verify_password(&current, user.base.password_hash.as_ref().unwrap())?;
        }

        user.set_password(hash_password(&new_password)?);

        // Invalidate all other sessions by bumping the session epoch. The
        // acting session is re-stamped with the new epoch by the handler.
        user.base.session_epoch += 1;

        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::PasswordChanged {
                    email: user.base.email.clone(),
                    had_password,
                    timestamp: Utc::now(),
                },
                authentication.clone(),
            ))
            .await?;

        self.user_service.update(&mut user, authentication).await
    }

    /// Initiate password reset process - generates a token stored in database
    pub async fn initiate_password_reset(
        &self,
        email: &EmailAddress,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        let mut user = match self
            .user_service
            .get_unique(StorableFilter::<User>::new_from_email(email))
            .await?
            .at_most_one()?
        {
            Some(user) => user,
            None => {
                // User doesn't exist - but we still return Ok to prevent enumeration
                tracing::info!("Password reset requested for non-existent email");
                return Ok(());
            }
        };

        // Generate token and store in database
        let token = Self::generate_secure_token();
        let expires = Utc::now() + Duration::hours(Self::PASSWORD_RESET_TOKEN_EXPIRY_HOURS);
        user.base.password_reset_token = Some(token.clone());
        user.base.password_reset_expires = Some(expires);
        self.user_service
            .update(&mut user, AuthenticatedEntity::System)
            .await?;

        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::PasswordResetRequested {
                    email_and_token: EmailAndToken {
                        email: user.base.email,
                        token,
                    },
                },
                AuthenticatedEntity::Anonymous,
            ))
            .await?;

        Ok(())
    }

    /// Reset password using token from database
    pub async fn complete_password_reset(
        &self,
        token: &str,
        new_password: &str,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        // Find user by password reset token
        let mut user = self
            .user_service
            .get_unique(StorableFilter::<User>::new_from_password_reset_token(token))
            .await?
            .at_most_one()?
            .ok_or_else(|| anyhow!("Invalid or expired password reset token"))?;

        // Check if token is expired
        if let Some(expires) = user.base.password_reset_expires {
            if Utc::now() > expires {
                // Clear expired token
                user.base.password_reset_token = None;
                user.base.password_reset_expires = None;
                self.user_service
                    .update(&mut user, AuthenticatedEntity::System)
                    .await?;
                return Err(anyhow!("Password reset token has expired"));
            }
        } else {
            return Err(anyhow!("Invalid password reset token"));
        }

        let authentication: AuthenticatedEntity = user.clone().into();
        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::PasswordResetCompleted,
                authentication,
            ))
            .await?;

        // Update password and clear token
        let hashed_password = hash_password(new_password)?;
        user.set_password(hashed_password);
        user.base.password_reset_token = None;
        user.base.password_reset_expires = None;
        // Invalidate all existing sessions (a reset implies the account may be
        // compromised). The reset handler re-stamps the requesting browser's
        // session with the new epoch.
        user.base.session_epoch += 1;
        self.user_service
            .update(&mut user, AuthenticatedEntity::System)
            .await?;

        Ok(user.clone())
    }

    pub async fn logout(
        &self,
        user_id: Uuid,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        if let Ok(Some(user)) = self.user_service.get_by_id(&user_id).await {
            let authentication: AuthenticatedEntity = user.into();
            self.event_bus
                .publish(Event::new(
                    AuthScope {
                        user_id: authentication.user_id(),
                        organization_id: authentication.organization_id(),
                        ip_address: ip,
                        user_agent,
                    },
                    AuthOperation::LoggedOut,
                    authentication,
                ))
                .await?;
        }

        Ok(())
    }

    /// Helper to generate verification token and set on user prior to emitting event that will send a verification email
    async fn set_user_email_verification_params(&self, user: &mut User) -> Result<EmailAndToken> {
        // Generate token and expiry
        let token = Self::generate_secure_token();
        let expiry = Utc::now() + Duration::hours(Self::VERIFICATION_TOKEN_EXPIRY_HOURS);

        // Store token in user record
        user.base.email_verification_token = Some(token.clone());
        user.base.email_verification_expires = Some(expiry);
        self.user_service
            .update(user, AuthenticatedEntity::System)
            .await?;

        Ok(EmailAndToken {
            token,
            email: user.base.email.clone(),
        })
    }

    /// Request an email change — sends verification email to the new address
    pub async fn request_email_change(
        &self,
        user_id: Uuid,
        current_password: Option<String>,
        new_email: EmailAddress,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        let mut user = self
            .user_service
            .get_by_id(&user_id)
            .await?
            .ok_or_else(|| anyhow!("User not found"))?;

        if user.base.oidc_provider.is_some() {
            return Err(anyhow!(
                "Cannot change email for accounts linked to an identity provider"
            ));
        }

        // Verify current password
        let current = current_password
            .ok_or_else(|| anyhow!("Current password is required to change your email"))?;
        verify_password(&current, user.base.password_hash.as_ref().unwrap())?;

        // Verify new email differs from current
        if new_email
            .as_ref()
            .eq_ignore_ascii_case(user.base.email.as_ref())
        {
            return Err(anyhow!(
                "New email must be different from your current email"
            ));
        }

        // Check new email isn't already taken
        let existing = self
            .user_service
            .get_all(StorableFilter::<User>::new_from_email(&new_email))
            .await?;
        if !existing.is_empty() {
            return Err(anyhow!("This email address is already in use"));
        }

        user.base.pending_email = Some(new_email.clone());

        let params = self.set_user_email_verification_params(&mut user).await?;

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
                AuthOperation::EmailChangeRequested {
                    // Send verification email to new address
                    email_and_token: EmailAndToken {
                        email: new_email,
                        token: params.token,
                    },
                },
                authentication,
            ))
            .await?;

        Ok(())
    }

    /// Verify email using token — handles both initial verification and email change flows
    pub async fn verify_email(
        &self,
        token: &str,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<User> {
        // Find user by verification token
        let mut user = self
            .user_service
            .get_unique(StorableFilter::<User>::new_from_email_verification_token(
                token,
            ))
            .await?
            .at_most_one()?
            .ok_or_else(|| anyhow!("Invalid verification token"))?;

        // Check if token is expired
        if let Some(expires) = user.base.email_verification_expires {
            if Utc::now() > expires {
                return Err(anyhow!(
                    "Verification token has expired. Please request a new one."
                ));
            }
        } else {
            return Err(anyhow!("Invalid verification token"));
        }

        // Check if this is an email change flow (pending_email is set)
        if let Some(pending_email) = user.base.pending_email.take() {
            let old_email = user.base.email.clone();
            user.base.email = pending_email;
            user.base.email_verified = true;
            user.base.email_verification_token = None;
            user.base.email_verification_expires = None;

            self.user_service
                .update(&mut user, AuthenticatedEntity::System)
                .await?;

            let new_email = user.base.email.clone();

            self.event_bus
                .publish(Event::new(
                    AuthScope {
                        user_id: Some(user.id),
                        organization_id: Some(user.base.organization_id),
                        ip_address: ip,
                        user_agent,
                    },
                    AuthOperation::EmailChanged {
                        old_email,
                        new_email,
                    },
                    user.clone().into(),
                ))
                .await?;
        } else {
            // Standard initial verification flow
            user.base.email_verified = true;
            user.base.email_verification_token = None;
            user.base.email_verification_expires = None;

            self.user_service
                .update(&mut user, AuthenticatedEntity::System)
                .await?;

            self.event_bus
                .publish(Event::new(
                    AuthScope {
                        user_id: Some(user.id),
                        organization_id: Some(user.base.organization_id),
                        ip_address: ip,
                        user_agent,
                    },
                    AuthOperation::EmailVerified,
                    user.clone().into(),
                ))
                .await?;
        }

        Ok(user)
    }

    /// Resend verification email with rate limiting
    pub async fn resend_verification_email(
        &self,
        email: &EmailAddress,
        ip: IpAddr,
        user_agent: Option<String>,
    ) -> Result<()> {
        // Check rate limiting
        {
            let cooldowns = self.verification_resend_cooldown.read().await;
            if let Some(last_sent) = cooldowns.get(email)
                && last_sent.elapsed().as_secs() < Self::RESEND_COOLDOWN_SECS
            {
                let remaining = Self::RESEND_COOLDOWN_SECS - last_sent.elapsed().as_secs();
                return Err(anyhow!(
                    "Please wait {} seconds before requesting another verification email",
                    remaining
                ));
            }
        }

        // Find user
        let mut user = self
            .user_service
            .get_unique(StorableFilter::<User>::new_from_email(email))
            .await?
            .at_most_one()?
            .ok_or_else(|| anyhow!("User not found"))?;

        // Check if already verified
        if user.base.email_verified {
            return Err(anyhow!("Email is already verified"));
        }

        let email_and_token = self.set_user_email_verification_params(&mut user).await?;
        let authentication: AuthenticatedEntity = user.clone().into();

        self.event_bus
            .publish(Event::new(
                AuthScope {
                    user_id: Some(user.id),
                    organization_id: Some(user.base.organization_id),
                    ip_address: ip,
                    user_agent,
                },
                AuthOperation::EmailVerificationRequested { email_and_token },
                authentication,
            ))
            .await?;

        // Update cooldown
        self.verification_resend_cooldown
            .write()
            .await
            .insert(email.clone(), Instant::now());

        Ok(())
    }

    /// Cleanup old login attempts (called periodically from background task)
    pub async fn cleanup_old_login_attempts(&self) {
        let mut attempts = self.login_attempts.write().await;

        attempts.retain(|_, (_, last_attempt)| {
            last_attempt.elapsed().as_secs() < Self::LOCKOUT_DURATION_SECS
        });

        tracing::debug!("Cleaned up old login attempts");
    }

    /// Cleanup old verification resend cooldowns
    pub async fn cleanup_old_verification_cooldowns(&self) {
        let mut cooldowns = self.verification_resend_cooldown.write().await;

        cooldowns
            .retain(|_, last_sent| last_sent.elapsed().as_secs() < Self::RESEND_COOLDOWN_SECS * 2);

        tracing::debug!("Cleaned up old verification cooldowns");
    }
}

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow!("Password hashing failed: {}", e))?
        .to_string();

    Ok(hash)
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> Result<()> {
    let parsed_hash =
        PasswordHash::new(hash).map_err(|e| anyhow!("Invalid password hash: {}", e))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| anyhow!("Invalid username or password"))
}
