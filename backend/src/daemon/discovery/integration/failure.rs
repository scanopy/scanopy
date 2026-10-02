//! Typed failures for a credential attempt.
//!
//! # Why these live in their own module
//!
//! The fields below are private, and that is the entire enforcement mechanism: an integration
//! cannot add a failure path without saying what type of failure it is, because the only way to
//! build one is a constructor that names an [`AttemptOutcome`].
//!
//! Rust privacy is *module and descendants*, so this only holds if the integrations are not
//! descendants of the module defining the types. Every integration — `integration::snmp`,
//! `integration::container`, `integration::unifi` — is a child of `integration`, so defining
//! these in `integration/mod.rs` left the struct-literal form
//! `ProbeFailure { outcome, message }` available to all of them and enforced nothing. Here they
//! are siblings, and the literal does not compile outside this file.
//!
//! Nothing automatic guards that placement, deliberately — a lexical scan for the literal is
//! fragile and buys false confidence. To re-verify by hand in half a minute, paste
//! `ProbeFailure { outcome: AttemptOutcome::Rejected, message: String::new() }` into any
//! integration and run `cargo check --lib`: it must fail with
//! `E0451: fields ... are private`. If it compiles, these types have drifted back into the
//! integrations' ancestry and classification is no longer forced.
//!
//! This is one half of the mechanism. The other is that an integration cannot *report* anything
//! either — the session's warning buffers are `pub(super)` to `service`, so the only route to the
//! operator is `DiscoveryOps`, which applies the reporting policy in one place. Both halves are
//! needed: classification without a single reporting path just means an integration classifies
//! correctly and then bypasses the renderer.

use crate::daemon::discovery::service::warnings::AttemptOutcome;

/// Failed probe, carrying what type of failure it was.
///
/// Only the integration knows whether its error was a refused password, a closed port or a TLS
/// problem, and nothing downstream can recover that from prose. Every one of these used to be a
/// bare string, so an operator was told "was rejected" whatever had happened — including when
/// they had cancelled the scan themselves.
pub struct ProbeFailure {
    outcome: AttemptOutcome,
    message: String,
}

impl ProbeFailure {
    fn new(outcome: AttemptOutcome, message: impl Into<String>) -> Self {
        Self {
            outcome,
            message: message.into(),
        }
    }

    /// The endpoint answered and refused the credential.
    pub fn rejected(message: impl Into<String>) -> Self {
        Self::new(AttemptOutcome::Rejected, message)
    }

    /// Nothing was listening — refused, or no route.
    pub fn unreachable(message: impl Into<String>) -> Self {
        Self::new(AttemptOutcome::Unreachable, message)
    }

    /// Connected, then no answer inside the timeout.
    pub fn timed_out(message: impl Into<String>) -> Self {
        Self::new(AttemptOutcome::TimedOut, message)
    }

    /// Something answered, but it is not this service.
    pub fn not_this_service(message: impl Into<String>) -> Self {
        Self::new(AttemptOutcome::NotThisService, message)
    }

    /// TLS negotiation failed.
    pub fn tls_failed(message: impl Into<String>) -> Self {
        Self::new(AttemptOutcome::TlsFailed, message)
    }

    /// The credential is incomplete or the wrong type for this integration.
    pub fn malformed(message: impl Into<String>) -> Self {
        Self::new(AttemptOutcome::Malformed, message)
    }

    /// The scan was cancelled. Never reported to the operator.
    pub fn cancelled() -> Self {
        Self::new(AttemptOutcome::Cancelled, "Discovery was cancelled")
    }

    /// For an integration whose client already classified the failure for itself.
    pub fn with_outcome(outcome: AttemptOutcome, message: impl Into<String>) -> Self {
        Self::new(outcome, message)
    }

    /// Classify a client-library error and keep its own wording.
    pub fn from_error<E>(error: &E) -> Self
    where
        for<'a> AttemptOutcome: From<&'a E>,
        E: std::fmt::Display,
    {
        Self::new(AttemptOutcome::from(error), error.to_string())
    }

    /// Add surrounding context without losing the classification.
    pub fn with_context(mut self, context: impl std::fmt::Display) -> Self {
        self.message = format!("{context}: {}", self.message);
        self
    }

    pub fn outcome(&self) -> AttemptOutcome {
        self.outcome
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// A failure during `execute()` — the credential worked and the collection after it did not.
///
/// Same shape and the same reason as [`ProbeFailure`]: an `execute` failure was previously only
/// `tracing::warn!`ed, so a Docker daemon that authenticated and then returned nothing produced a
/// host with unclaimed open ports and no services, and nothing anywhere said why.
pub struct IntegrationFailure {
    outcome: AttemptOutcome,
    message: String,
}

impl IntegrationFailure {
    /// The default for this phase: authentication already succeeded, so a failure here is about
    /// the collection rather than the credential.
    pub fn collection_failed(message: impl Into<String>) -> Self {
        Self {
            outcome: AttemptOutcome::CollectionFailed,
            message: message.into(),
        }
    }

    /// The collection ran out of time rather than failing.
    ///
    /// The integration's own cap fired, so the credential and the service are both fine and the
    /// only thing missing is time. Kept separate from [`Self::collection_failed`] because the
    /// operator's fix is different — narrow the scan or rescan the host alone, rather than go
    /// looking for a broken endpoint.
    pub fn collection_timed_out(message: impl Into<String>) -> Self {
        Self {
            outcome: AttemptOutcome::CollectionTimedOut,
            message: message.into(),
        }
    }

    /// For the cases where the collection phase can establish something more specific — SNMP
    /// re-opening its session, for instance, which can time out on its own.
    pub fn with_outcome(outcome: AttemptOutcome, message: impl Into<String>) -> Self {
        Self {
            outcome,
            message: message.into(),
        }
    }

    pub fn cancelled() -> Self {
        Self::with_outcome(AttemptOutcome::Cancelled, "Discovery was cancelled")
    }

    pub fn outcome(&self) -> AttemptOutcome {
        self.outcome
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for IntegrationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Anything an integration already returns as an `anyhow::Error` degrades to the generic
/// collection failure rather than forcing every `?` to be rewritten.
///
/// A deliberate weakening, and the one place the mechanism is a default rather than a
/// requirement: the probe phase is where the operator-actionable distinctions live (refused,
/// TLS, wrong port), and by the time `execute` runs the credential has already worked. An
/// integration that can say something sharper uses [`IntegrationFailure::with_outcome`].
impl From<anyhow::Error> for IntegrationFailure {
    fn from(error: anyhow::Error) -> Self {
        Self::collection_failed(error.to_string())
    }
}

impl std::fmt::Display for ProbeFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
