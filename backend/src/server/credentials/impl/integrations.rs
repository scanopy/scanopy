//! The `integrations` aggregate: a website-facing view that joins a
//! [`ServiceDefinition`] (logo, name) with the [`CredentialType`]s that target it
//! (its transports) and one canonical "what's discovered" description.
//!
//! An "integration" is defined entirely by the credentials that point at a
//! service: it is the set of distinct `associated_service()` values over the
//! credential discriminants. There is no parallel store and no runtime endpoint —
//! [`all_integrations`] is computed from the existing credential/service metadata
//! and emitted as the `integrations.json` fixture by `generate-fixtures`, then
//! synced to the website the same way `service-definitions.json` is.

use serde::Serialize;
use strum::IntoEnumIterator;

use crate::server::services::r#impl::definitions::{ServiceDefinition, ServiceDefinitionExt};
use crate::server::shared::fixtures::logo_slug;
use crate::server::shared::types::field_definition::FieldDefinition;
use crate::server::shared::types::metadata::{EntityMetadataProvider, HasId, TypeMetadataProvider};
use crate::server::shared::types::{Color, Icon};

use super::types::{CredentialStability, CredentialTypeDiscriminants, Target, UpstreamSupport};

/// A kind of host virtualization an integration reports, by id (`HostVirtualization`'s tag) and
/// display name.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationVirtualization {
    pub id: String,
    pub name: String,
}

/// One integration: a service plus the transports (credential types) that reach
/// it, with a single canonical discovery description and a one-line summary.
#[derive(Debug, Clone, Serialize)]
pub struct Integration {
    /// Service id (e.g. "Docker", "Podman", "SNMP").
    pub id: String,
    pub name: String,
    /// Credential category grouping (e.g. "Container & Virtualization").
    pub category: String,
    pub has_logo: bool,
    /// File extension of the service logo (matches the downloaded
    /// `logos/services/{logo_slug}.{logo_ext}`). Empty when there is no logo.
    pub logo_ext: String,
    /// Filename stem of the service logo under `logos/services/`.
    pub logo_slug: String,
    pub logo_needs_white_background: bool,
    /// Whether the service is a protocol rather than a vendor's product (SNMP, SSH).
    /// Generic integrations sort after vendor ones.
    pub is_generic: bool,
    /// Lucide icon shown in place of the logo when `has_logo` is false.
    pub icon: Icon,
    /// Colour the app tints that fallback icon with.
    pub color: Color,
    /// Canonical "what's discovered" text — the single source shared by every
    /// transport of this integration.
    pub discovers: String,
    /// Path to this integration's documentation guide, relative to the site root and with a
    /// trailing slash. Shared by every transport, like `discovers`, because a guide documents the
    /// integration rather than one of its transports.
    pub docs_path: String,
    /// The kinds of host virtualization this integration reports
    /// ([`super::types::CredentialIntegration::host_virtualizations`]), with their display names,
    /// so the docs can list which integrations report which relationship without a fixture of
    /// their own.
    pub host_virtualizations: Vec<IntegrationVirtualization>,
    /// One-line summary: the discovery text plus the available transports.
    pub summary: String,
    /// Maturity and daemon-version floors are deliberately *not* summarized here.
    /// They vary by transport — SNMP v2c reaches a 0.16.2 daemon while v1/v3 need
    /// 0.17.0 — so an integration-level number would be wrong for some of its own
    /// rows. Read them from each [`IntegrationTransport`].
    pub transports: Vec<IntegrationTransport>,
}

/// One transport (credential type) of an integration.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationTransport {
    /// Credential type discriminant (e.g. "DockerSocket").
    pub id: String,
    /// Short transport label (e.g. "Socket", "Proxy", "v2c").
    pub name: String,
    /// Full credential-type name as the app shows it (e.g. "UniFi API Key").
    ///
    /// Not `"{integration name} {transport name}"`: consumers were composing that
    /// themselves, which is right only where the service name is also the
    /// credential's prefix. It isn't for UniFi — the service is "UniFi Controller"
    /// but the credential is "UniFi API Key" — so the composed string named a type
    /// that does not exist in the app.
    pub display_name: String,
    /// Transport-specific note, derived from the credential's `transport_note`.
    pub description: String,
    pub requires_config: bool,
    pub single_endpoint_per_host: bool,
    /// Where this transport can be applied (daemon host, hosts, network).
    pub targets: Vec<Target>,
    /// Release maturity of this specific transport.
    pub stability: CredentialStability,
    /// Whether the vendor publishes the API this transport talks to. Independent of
    /// `stability`: a transport can be fully validated (`Stable`) and still ride an
    /// endpoint the vendor never documented, which is true of both UniFi transports.
    pub upstream_support: UpstreamSupport,
    /// Minimum daemon version that can receive this transport.
    pub minimum_daemon_version: semver::Version,
    /// What the operator has to fill in, in the order the form asks for it. The
    /// same definitions the app builds its credential form from, so a documented
    /// field and the app's own label/help text cannot drift apart.
    pub fields: Vec<FieldDefinition>,
}

/// Build every integration by grouping credential discriminants on their
/// associated service. Output is deterministic (vendor integrations before generic
/// ones, then by integration id, then transport id) so re-running
/// `generate-fixtures` yields no diff.
pub fn all_integrations() -> Vec<Integration> {
    let mut integrations: Vec<Integration> = Vec::new();

    for disc in CredentialTypeDiscriminants::iter() {
        let ct = disc.to_credential_type();
        let service = ct.associated_service();
        let service_id = ServiceDefinition::name(&*service).to_string();

        let transport = IntegrationTransport {
            id: <&'static str>::from(disc).to_string(),
            name: disc.transport_label().to_string(),
            display_name: disc.display_name().to_string(),
            description: disc.transport_note().to_string(),
            requires_config: ct.requires_config(),
            single_endpoint_per_host: ct.single_endpoint_per_host(),
            targets: ct.targets(),
            stability: disc.stability(),
            upstream_support: disc.upstream_support(),
            minimum_daemon_version: disc.minimum_daemon_version(),
            // Form order, not sorted: it is the order the fields are meant to be
            // read in, and it is already deterministic.
            fields: ct.field_definitions(),
        };

        if let Some(existing) = integrations.iter_mut().find(|i| i.id == service_id) {
            existing.transports.push(transport);
            continue;
        }

        integrations.push(Integration {
            id: service_id,
            name: ServiceDefinition::name(&*service).to_string(),
            category: <&'static str>::from(ct.credential_category()).to_string(),
            has_logo: service.has_logo(),
            logo_ext: disc.integration().logo_ext().to_string(),
            logo_slug: logo_slug(ServiceDefinition::name(&*service)),
            logo_needs_white_background: service.logo_needs_white_background(),
            is_generic: ServiceDefinition::is_generic(&*service),
            icon: disc.integration().icon(),
            color: disc.integration().color(),
            discovers: disc.integration().discovers().to_string(),
            docs_path: disc.integration().docs_path().to_string(),
            host_virtualizations: disc
                .integration()
                .host_virtualizations()
                .iter()
                .map(|v| IntegrationVirtualization {
                    id: v.id().to_string(),
                    name: TypeMetadataProvider::name(v).to_string(),
                })
                .collect(),
            summary: String::new(), // filled in after all transports are collected
            transports: vec![transport],
        });
    }

    for integration in &mut integrations {
        integration.transports.sort_by(|a, b| a.id.cmp(&b.id));
        let labels: Vec<String> = integration
            .transports
            .iter()
            .map(|t| t.name.to_lowercase())
            .collect();
        integration.summary = format!(
            "{} Supports {}.",
            integration.discovers,
            join_oxford(&labels)
        );
    }

    // Vendor integrations first, generic protocols (SNMP, SSH, gNMI) after.
    integrations.sort_by(|a, b| (a.is_generic, &a.id).cmp(&(b.is_generic, &b.id)));
    integrations
}

/// Oxford-comma join: `["a"] -> "a"`, `["a","b"] -> "a and b"`,
/// `["a","b","c"] -> "a, b, and c"`.
fn join_oxford(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!("{a} and {b}"),
        [rest @ .., last] => format!("{}, and {}", rest.join(", "), last),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generic structural test (no per-integration assertions): every integration
    /// is well-formed and its credential descriptions derive from the centralized
    /// text. Mirrors the spirit of the service-definition specificity tests.
    #[test]
    fn integrations_are_well_formed() {
        let integrations = all_integrations();
        assert!(
            !integrations.is_empty(),
            "expected at least one integration"
        );

        for integration in &integrations {
            assert!(!integration.id.is_empty(), "integration id is empty");
            assert!(
                !integration.discovers.trim().is_empty(),
                "{} has no discovery text",
                integration.id
            );
            assert!(
                !integration.summary.trim().is_empty(),
                "{} has no summary",
                integration.id
            );
            assert!(
                integration.summary.contains(&integration.discovers),
                "{} summary should build on its discovery text",
                integration.id
            );
            // Root-relative with a trailing slash: the website joins it onto its own origin and
            // the docs site serves every page with one, so a bare or absolute path would 404.
            assert!(
                integration.docs_path.starts_with("/docs/") && integration.docs_path.ends_with('/'),
                "{} docs path should be a root-relative /docs/… path with a trailing slash, got {:?}",
                integration.id,
                integration.docs_path
            );
            assert!(
                !integration.transports.is_empty(),
                "{} has no transports",
                integration.id
            );

            for transport in &integration.transports {
                assert!(
                    !transport.display_name.trim().is_empty(),
                    "{}/{} has no display name",
                    integration.id,
                    transport.id
                );
                // A 0.0.0 floor means the field was defaulted rather than projected
                // from `minimum_daemon_version()`.
                assert!(
                    transport.minimum_daemon_version > semver::Version::new(0, 0, 0),
                    "{}/{} has no daemon version floor",
                    integration.id,
                    transport.id
                );
                assert!(
                    !transport.requires_config || !transport.fields.is_empty(),
                    "{}/{} requires configuration but projects no fields",
                    integration.id,
                    transport.id
                );

                let mut ids: Vec<&str> = transport.fields.iter().map(|f| f.id).collect();
                let count = ids.len();
                ids.sort_unstable();
                ids.dedup();
                assert_eq!(
                    ids.len(),
                    count,
                    "{}/{} has duplicate field ids",
                    integration.id,
                    transport.id
                );
            }

            // Transports of one integration must be distinguishable.
            let mut notes: Vec<&str> = integration
                .transports
                .iter()
                .map(|t| t.description.as_str())
                .collect();
            notes.sort_unstable();
            let unique = notes.len();
            notes.dedup();
            assert_eq!(
                notes.len(),
                unique,
                "{} has duplicate transport descriptions",
                integration.id
            );
        }
    }

    /// Locks the de-duplication: each credential's full description is exactly the
    /// canonical discovery text + its transport note (single source, derived).
    #[test]
    fn credential_description_derives_from_canonical_text() {
        for disc in CredentialTypeDiscriminants::iter() {
            let expected = format!(
                "{} {}",
                disc.integration().discovers(),
                disc.transport_note()
            );
            assert_eq!(disc.full_description(), expected);
            // The shared stem must actually be the leading text.
            assert!(
                disc.full_description()
                    .starts_with(disc.integration().discovers())
            );
        }
    }
}
