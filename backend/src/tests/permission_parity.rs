//! Temporary: the status every principal gets from every authenticated route, written to the file
//! `PARITY_OUT` names. Run before and after handlers move to `Can<T, A>`, then diff the two files.
//! Deleted once the diff equals the plan's "differs from today" table.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Value, json};
use strum::IntoEnumIterator;
use uuid::Uuid;

use super::cross_tenant_probe::{Route, route_entity, rows_of, serve_as};
use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{site, subnet, user};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::config::AppState;
use crate::server::invites::r#impl::base::{Invite, InviteBase};
use crate::server::organizations::demo_data::DemoData;
use crate::server::organizations::demo_seed::insert_demo_data;
use crate::server::shared::api_key_common::{ApiKeyType, generate_api_key_for_storage};
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::entities::EntityDiscriminants as E;
use crate::server::shared::services::entity_fields::ServiceSource;
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::filter::StorableFilter;
use crate::server::shared::storage::traits::Storable;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use crate::server::user_api_keys::r#impl::base::{UserApiKey, UserApiKeyBase};
use crate::server::users::r#impl::permissions::UserOrgPermissions as Role;

#[derive(Clone, Copy)]
enum Principal {
    User {
        api_key: bool,
        /// Holds one site with no seeded rows, so every seeded site is out of scope.
        limited: bool,
        role: Role,
    },
    Daemon,
}

impl Principal {
    fn label(&self) -> String {
        match self {
            Principal::User {
                api_key,
                limited,
                role,
            } => format!(
                "{}-{}-{role}",
                if *api_key { "key" } else { "user" },
                if *limited { "limited" } else { "full" },
            ),
            Principal::Daemon => "daemon".to_string(),
        }
    }

    fn role(&self) -> Role {
        match self {
            Principal::User { role, .. } => *role,
            Principal::Daemon => Role::Viewer,
        }
    }
}

/// Routes that take no authorization: auth flows, webhooks and public reads.
const PUBLIC: &[&str] = &[
    "/api/auth/",
    "/api/billing/webhooks",
    "/api/config",
    "/api/email/",
    "/api/github-stars",
    "/api/install/",
    "/api/v1/licenses/entitlement",
    "/api/v1/shares/public/",
    "/api/version",
];

/// Destructive routes run last, so they don't remove the rows the others are called with.
fn order(method: &str, path: &str) -> u8 {
    let bulk_delete = path.ends_with("/bulk-delete");
    match method {
        _ if path.contains("/organizations/") && (path.ends_with("/reset") || method == "delete") => {
            7
        }
        "delete" | "post" if path.contains("/sites") && (method == "delete" || bulk_delete) => 6,
        "delete" => 5,
        _ if bulk_delete => 4,
        "get" => 0,
        "put" => 1,
        _ => 2,
    }
}

async fn walk(
    state: &Arc<AppState>,
    principal: Principal,
    spec: &Value,
    client: &reqwest::Client,
) -> Vec<String> {
    let label = principal.label();
    let role = principal.role();
    let org = create_org(state, get_enterprise_plan(), None).await;

    let mut owner = user(&org.id);
    owner.base.email = format!("owner-{label}@example.com").parse().unwrap();
    owner.base.permissions = Role::Owner;
    let owner = state
        .services
        .user_service
        .create(owner, AuthenticatedEntity::System)
        .await
        .unwrap();
    let demo_data = DemoData::generate(org.id, owner.id, Default::default());
    insert_demo_data(
        &state.services,
        demo_data,
        org.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .expect("demo seeding succeeds");

    let extra = state
        .services
        .site_service
        .create(site(&org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let seeded_sites: Vec<Uuid> = state
        .services
        .site_service
        .get_all(StorableFilter::new_from_org_id(&org.id))
        .await
        .unwrap()
        .iter()
        .map(|s| s.id)
        .filter(|id| *id != extra.id)
        .collect();
    assert!(!seeded_sites.is_empty());

    let mut caller = user(&org.id);
    caller.base.email = format!("caller-{label}@example.com").parse().unwrap();
    caller.base.permissions = role;
    let caller = state
        .services
        .user_service
        .create(caller, AuthenticatedEntity::System)
        .await
        .unwrap();
    let limited = matches!(principal, Principal::User { limited: true, .. });
    let site_ids: Vec<Uuid> = if limited {
        vec![extra.id]
    } else {
        seeded_sites.iter().copied().chain([extra.id]).collect()
    };
    let (_, hashed) = generate_api_key_for_storage(ApiKeyType::User);
    let own_key = state
        .services
        .user_api_key_service
        .create_with_sites(
            UserApiKey::new(UserApiKeyBase {
                key: hashed,
                name: "own".to_string(),
                user_id: caller.id,
                organization_id: org.id,
                permissions: role,
                is_enabled: true,
                ..UserApiKeyBase::default()
            }),
            site_ids.clone(),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();
    state
        .services
        .invite_service
        .create(
            Invite::new(InviteBase {
                organization_id: org.id,
                permissions: Role::Viewer,
                url: "https://example.com/invite".to_string(),
                created_by: owner.id,
                expires_at: chrono::Utc::now() + chrono::Duration::days(7),
                ..InviteBase::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let source = ServiceSource {
        services: &state.services,
        site_ids: &seeded_sites,
        organization_id: org.id,
    };
    let mut rows: HashMap<E, Vec<Value>> = HashMap::new();
    for entity in E::iter() {
        rows.insert(entity, rows_of(&source, entity, org.id, &seeded_sites).await);
    }
    rows.insert(E::Organization, vec![serde_json::to_value(&org).unwrap()]);
    // The caller's own key first, because only the key's own user reaches it.
    rows.get_mut(&E::UserApiKey)
        .unwrap()
        .sort_by_key(|row| row["id"] != json!(own_key.id));
    // Someone else first: `GET /users/{id}` and the rank checks mean nothing on the caller.
    rows.get_mut(&E::User)
        .unwrap()
        .sort_by_key(|row| row["id"] == json!(caller.id));
    let cidr = |value: &str| {
        SubnetCidr::new(
            SubnetCidrValue(value.parse().unwrap()),
            AttributeSource::DaemonSelfReport,
        )
    };
    let mut contained = subnet(&seeded_sites[0]);
    contained.base.cidr = cidr("10.77.1.0/24");
    let mut covering = subnet(&seeded_sites[0]);
    covering.base.cidr = cidr("10.77.0.0/16");
    for (position, row) in [contained, covering].into_iter().enumerate() {
        let created = state
            .services
            .subnet_service
            .create(row, AuthenticatedEntity::System)
            .await
            .unwrap();
        rows.get_mut(&E::Subnet)
            .unwrap()
            .insert(position, serde_json::to_value(&created).unwrap());
    }

    let entity = match principal {
        Principal::User { api_key: true, .. } => AuthenticatedEntity::ApiKey {
            api_key_id: own_key.id,
            user_id: caller.id,
            organization_id: org.id,
            permissions: role,
            site_ids: site_ids.clone(),
        },
        Principal::User { .. } => AuthenticatedEntity::User {
            user_id: caller.id,
            organization_id: org.id,
            permissions: role,
            site_ids: site_ids.clone(),
            email: caller.base.email.clone(),
            email_verified: true,
            terms_accepted_at: caller.base.terms_accepted_at,
        },
        Principal::Daemon => {
            let daemon = &rows[&E::Daemon][0];
            AuthenticatedEntity::Daemon {
                site_id: serde_json::from_value(daemon["site_id"].clone()).unwrap(),
                api_key_id: Uuid::new_v4(),
                daemon_id: serde_json::from_value(daemon["id"].clone()).unwrap(),
                version: None,
            }
        }
    };
    let addr = serve_as(state, entity).await;

    let schemas = &spec["components"]["schemas"];
    let mut routes: Vec<(String, String, &Value)> = Vec::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        if PUBLIC.iter().any(|prefix| path.starts_with(prefix)) {
            continue;
        }
        for (method, operation) in item.as_object().unwrap() {
            routes.push((method.clone(), path.clone(), operation));
        }
    }
    routes.sort_by_key(|(method, path, _)| order(method, path));

    // What a route's own check decides from the body, which the walk's single body can't reach.
    let mut lines = Vec::new();
    let post = |path: &str, body: Value| {
        client
            .post(format!("http://{addr}{path}"))
            .json(&body)
            .send()
    };
    for invited in Role::iter() {
        let response = post(
            "/api/v1/invites",
            json!({"permissions": invited, "site_ids": []}),
        )
        .await;
        let status = response.map(|r| r.status().as_u16()).unwrap_or(0);
        lines.push(format!("{label}\tinvite as {invited}\t[{status}]"));
    }
    for entity in E::iter().filter(E::is_taggable) {
        for (position, row) in rows[&entity].iter().take(2).enumerate() {
            let response = client
                .put(format!("http://{addr}/api/v1/tags/assign"))
                .json(&json!({"entity_type": entity, "entity_id": row["id"], "tag_ids": []}))
                .send()
                .await;
            let status = response.map(|r| r.status().as_u16()).unwrap_or(0);
            lines.push(format!("{label}\ttag {entity} {position}\t[{status}]"));
        }
    }

    let nil = vec![json!({"id": Uuid::nil()})];
    for (method, path, operation) in &routes {
        let route_rows = route_entity(operation)
            .and_then(|entity| rows.get(&entity))
            .filter(|rows| !rows.is_empty())
            .unwrap_or(&nil);
        let route = Route {
            method,
            path,
            operation,
            schemas,
            rows: route_rows,
        };
        let statuses = route.call(client, addr, extra.id).await;
        lines.push(format!("{label}\t{method} {path}\t{statuses:?}"));
    }
    lines
}

#[tokio::test]
#[ignore = "temporary: writes the parity snapshot to PARITY_OUT"]
async fn parity_snapshot() {
    let out = std::env::var("PARITY_OUT").expect("PARITY_OUT names the output file");
    let spec = serde_json::to_value(crate::server::openapi::full_spec()).unwrap();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap();

    let mut principals = vec![Principal::Daemon];
    for role in [Role::Viewer, Role::Member, Role::Admin, Role::Owner] {
        for api_key in [false, true] {
            principals.push(Principal::User {
                api_key,
                limited: false,
                role,
            });
            // Admin and Owner users hold every site by rule, so only keys come limited.
            if api_key || role < Role::Admin {
                principals.push(Principal::User {
                    api_key,
                    limited: true,
                    role,
                });
            }
        }
    }

    let mut lines = Vec::new();
    for principal in principals {
        // Demo rows have fixed ids, so each principal gets its own database.
        let (state, _container) = cloud_test_state().await;
        lines.extend(walk(&state, principal, &spec, &client).await);
    }
    lines.sort();
    std::fs::write(out, lines.join("\n") + "\n").unwrap();
}
