//! One organization's owner calls every route that takes an id, using ids from another
//! organization, and no call may succeed. The routes come from the OpenAPI spec, so a new route is
//! probed without anyone adding it here.

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::middleware::{self, Next};
use serde_json::{Value, json};
use strum::IntoEnumIterator;
use uuid::Uuid;

use super::self_hosted_licensing::{cloud_test_state, create_org};
use super::{site, subnet, user};
use crate::server::auth::access::{Keying, RECORDED_CHECKS, Resource};
use crate::server::auth::middleware::auth::AuthenticatedEntity;
use crate::server::billing::plans::get_enterprise_plan;
use crate::server::config::AppState;
use crate::server::invites::r#impl::base::{Invite, InviteBase};
use crate::server::organizations::demo_data::DemoData;
use crate::server::organizations::demo_seed::insert_demo_data;
use crate::server::shared::attribution::AttributeSource;
use crate::server::shared::entities::EntityDiscriminants as E;
use crate::server::shared::handlers::factory::{create_public_share_routes, create_router};
use crate::server::shared::services::entity_fields::{RelatedSource, ServiceSource};
use crate::server::shared::services::traits::CrudService;
use crate::server::shared::storage::traits::Storable;
use crate::server::subnets::r#impl::base::{SubnetCidr, SubnetCidrValue};
use crate::server::users::r#impl::permissions::UserOrgPermissions;

/// Routes where another organization's id is meant to work, or that hold no row the probe can
/// name, and why.
const EXEMPT: &[(&str, &str)] = &[
    (
        "/api/v1/shares/public/",
        "public by design: the id is the capability",
    ),
    (
        "/api/v1/invites/{id}/accept",
        "the invite id is the capability",
    ),
    (
        "/api/v1/licenses/instances/{id}",
        "no row without a server check-in; `license_instance_service.delete` keys on the org too",
    ),
];

/// Reads the rows' own owner can't make either, so the control can't vouch for them, and why.
const CONTROL_EXEMPT: &[(&str, &str)] = &[
    (
        "/api/v1/daemons/{id}/install-command",
        "refused for a daemon that isn't awaiting install",
    ),
    (
        "/api/v1/organizations/{id}/populate-demo/status",
        "404 until a demo population has run",
    ),
    (
        "/api/v1/user-api-keys/{id}",
        "only the key's own user reads it, and the demo key belongs to the demo login",
    ),
];

/// Routes that take no authorization: auth flows, webhooks and public reads.
pub(super) const PUBLIC: &[&str] = &[
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

/// The response header carrying the checks a request made, set by [`serve_as`].
const RECORDED_CHECKS_HEADER: &str = "x-recorded-checks";

/// Entities keyed by organization rather than site.
const ORG_KEYED: &[E] = &[
    E::Tag,
    E::TagGroup,
    E::User,
    E::UserApiKey,
    E::Credential,
    E::Invite,
];

/// Serve the whole API with `caller` as the authenticated entity of every request.
pub(super) async fn serve_as(state: &Arc<AppState>, caller: AuthenticatedEntity) -> SocketAddr {
    let (router, _) = create_router(state.clone());
    let (public_shares, _) = create_public_share_routes().split_for_parts();
    let app = router
        .merge(public_shares)
        .layer(axum_client_ip::ClientIpSource::ConnectInfo.into_extension())
        .layer(middleware::from_fn(
            move |mut request: Request<Body>, next: Next| {
                let caller = caller.clone();
                async move {
                    request.extensions_mut().insert(caller);
                    RECORDED_CHECKS
                        .scope(RefCell::default(), async {
                            let mut response = next.run(request).await;
                            let checks = RECORDED_CHECKS.with(|checks| {
                                checks
                                    .borrow()
                                    .iter()
                                    .map(|(resource, action)| format!("{resource}:{action}"))
                                    .collect::<Vec<_>>()
                                    .join(",")
                            });
                            response
                                .headers_mut()
                                .insert(RECORDED_CHECKS_HEADER, checks.parse().unwrap());
                            response
                        })
                        .await
                }
            },
        ))
        .with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap()
    });
    addr
}

/// Every row of `entity` in the organization, serialized.
pub(super) async fn rows_of(
    source: &ServiceSource<'_>,
    entity: E,
    organization_id: Uuid,
    site_ids: &[Uuid],
) -> Vec<Value> {
    match entity {
        E::Organization => vec![],
        E::Site => source
            .by_column(entity, "id", site_ids)
            .await
            .unwrap_or_default(),
        _ if ORG_KEYED.contains(&entity) => source
            .by_column(entity, "organization_id", &[organization_id])
            .await
            .unwrap_or_default(),
        _ => source
            .by_column(entity, "site_id", site_ids)
            .await
            .unwrap_or_default(),
    }
}

/// The entity a route operates on: the type named by its first OpenAPI tag.
pub(super) fn route_entity(operation: &Value) -> Option<E> {
    let tags = operation["tags"].as_array()?;
    E::iter().find(|entity| {
        tags.iter()
            .any(|tag| tag.as_str() == Some(entity.entity_name_plural()))
            || tags
                .iter()
                .any(|tag| tag.as_str().is_some_and(|t| t == entity.as_ref()))
    })
}

fn resolve<'a>(schemas: &'a Value, schema: &'a Value) -> &'a Value {
    match schema.get("$ref").and_then(Value::as_str) {
        Some(reference) => &schemas[reference.rsplit('/').next().unwrap()],
        None => schema,
    }
}

/// A body the schema accepts: each uuid is `id`, except `site_id`, which is the caller's own.
fn sample(schemas: &Value, schema: &Value, id: Uuid, own_site: Uuid, depth: u8) -> Value {
    let schema = resolve(schemas, schema);
    if depth > 8 {
        return Value::Null;
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return values.first().cloned().unwrap_or(Value::Null);
    }
    for key in ["allOf", "oneOf", "anyOf"] {
        if let Some(part) = schema
            .get(key)
            .and_then(Value::as_array)
            .and_then(|parts| parts.iter().find(|p| p.get("type") != Some(&json!("null"))))
        {
            return sample(schemas, part, id, own_site, depth + 1);
        }
    }
    let type_name = match schema.get("type") {
        Some(Value::Array(types)) => types
            .iter()
            .find(|t| *t != "null")
            .and_then(Value::as_str)
            .unwrap_or("null"),
        Some(value) => value.as_str().unwrap_or("object"),
        None => "object",
    };
    match type_name {
        "object" => {
            let mut object = serde_json::Map::new();
            let required = schema.get("required").and_then(Value::as_array);
            for key in required.into_iter().flatten().filter_map(Value::as_str) {
                let value = if key == "site_id" {
                    json!(own_site)
                } else {
                    sample(schemas, &schema["properties"][key], id, own_site, depth + 1)
                };
                object.insert(key.to_string(), value);
            }
            Value::Object(object)
        }
        "array" => json!([sample(schemas, &schema["items"], id, own_site, depth + 1)]),
        "integer" | "number" => json!(0),
        "boolean" => json!(false),
        "null" => Value::Null,
        _ => match schema.get("format").and_then(Value::as_str) {
            Some("uuid") => json!(id),
            Some("date-time") => json!(chrono::Utc::now()),
            _ => json!("x"),
        },
    }
}

/// The bodies to try on a route: the row itself, the row claiming the caller's site, and a body
/// built from the schema.
fn bodies(
    schemas: &Value,
    operation: &Value,
    row: &Value,
    other: &Value,
    own_site: Uuid,
) -> Vec<Option<Value>> {
    let schema = &operation["requestBody"]["content"]["application/json"]["schema"];
    if schema.is_null() {
        return vec![None];
    }
    let id: Uuid = serde_json::from_value(other["id"].clone()).unwrap_or_default();
    let mut moved = row.clone();
    if moved.get("site_id").is_some() {
        moved["site_id"] = json!(own_site);
    }
    vec![
        Some(row.clone()),
        Some(moved),
        Some(sample(schemas, schema, id, own_site, 0)),
    ]
}

/// An organization with demo data, an invite and a pair of mergeable subnets, as its owner sees it.
struct SeededOrg {
    id: Uuid,
    /// The owner, as an authenticated entity.
    owner: AuthenticatedEntity,
    site_ids: Vec<Uuid>,
    rows: HashMap<E, Vec<Value>>,
}

async fn seed_org(state: &Arc<AppState>) -> SeededOrg {
    let target = create_org(state, get_enterprise_plan(), None).await;
    let owner = state
        .services
        .user_service
        .create(user(&target.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let demo_data = DemoData::generate(target.id, owner.id, Default::default());
    insert_demo_data(
        &state.services,
        demo_data,
        target.id,
        owner.id,
        AuthenticatedEntity::System,
    )
    .await
    .expect("demo seeding succeeds");

    state
        .services
        .invite_service
        .create(
            Invite::new(InviteBase {
                organization_id: target.id,
                permissions: UserOrgPermissions::Member,
                url: "https://example.com/invite".to_string(),
                created_by: owner.id,
                expires_at: chrono::Utc::now() + chrono::Duration::days(7),
                ..InviteBase::default()
            }),
            AuthenticatedEntity::System,
        )
        .await
        .unwrap();

    let site_ids: Vec<Uuid> = state
        .services
        .site_service
        .get_all(
            crate::server::shared::storage::filter::StorableFilter::new_from_org_id(&target.id),
        )
        .await
        .unwrap()
        .iter()
        .map(|s| s.id)
        .collect();
    assert!(!site_ids.is_empty(), "the demo seeds sites");
    let owner = AuthenticatedEntity::User {
        user_id: owner.id,
        organization_id: target.id,
        permissions: UserOrgPermissions::Owner,
        site_ids: site_ids.clone(),
        email: owner.base.email.clone(),
        email_verified: true,
        terms_accepted_at: owner.base.terms_accepted_at,
    };
    let source = ServiceSource {
        services: &state.services,
        reader: &owner,
        organization_id: target.id,
    };
    let mut rows: HashMap<E, Vec<Value>> = HashMap::new();
    for entity in E::iter() {
        rows.insert(entity, rows_of(&source, entity, target.id, &site_ids).await);
    }
    rows.insert(
        E::Organization,
        vec![serde_json::to_value(&target).unwrap()],
    );
    // A pair `POST /subnets/{id}/merge` accepts: the first row sits inside the second.
    let cidr = |value: &str| {
        SubnetCidr::new(
            SubnetCidrValue(value.parse().unwrap()),
            AttributeSource::DaemonSelfReport,
        )
    };
    let mut contained = subnet(&site_ids[0]);
    contained.base.cidr = cidr("10.77.1.0/24");
    let mut covering = subnet(&site_ids[0]);
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

    SeededOrg {
        id: target.id,
        owner,
        site_ids,
        rows,
    }
}

/// The spec's operations with deletes last, so a hole in one doesn't remove the row the others are
/// probed with.
fn operations(spec: &Value) -> Vec<(String, String, &Value)> {
    let mut routes: Vec<(String, String, &Value)> = Vec::new();
    for (path, item) in spec["paths"].as_object().unwrap() {
        for (method, operation) in item.as_object().unwrap() {
            routes.push((method.clone(), path.clone(), operation));
        }
    }
    routes.sort_by_key(|(method, _, _)| method == "delete");
    routes
}

/// Whether a route names a row of its entity: by id in the path, or in a bulk body.
fn names_rows(path: &str, operation: &Value) -> bool {
    let has_path_id = operation["parameters"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|p| p["in"] == "path" && p["schema"]["format"] == "uuid");
    has_path_id || (path.contains("/bulk-") && !operation["requestBody"].is_null())
}

#[tokio::test]
async fn no_route_accepts_another_organizations_ids() {
    let (state, _container) = cloud_test_state().await;
    let SeededOrg {
        owner,
        site_ids: target_sites,
        rows,
        ..
    } = seed_org(&state).await;

    let attacker_org = create_org(&state, get_enterprise_plan(), None).await;
    let mut attacker = user(&attacker_org.id);
    attacker.base.email = "attacker@example.com".parse().unwrap();
    let attacker = state
        .services
        .user_service
        .create(attacker, AuthenticatedEntity::System)
        .await
        .unwrap();
    let own_site = state
        .services
        .site_service
        .create(site(&attacker_org.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let addr = serve_as(
        &state,
        AuthenticatedEntity::User {
            user_id: attacker.id,
            organization_id: attacker_org.id,
            permissions: UserOrgPermissions::Owner,
            site_ids: vec![own_site.id],
            email: attacker.base.email.clone(),
            email_verified: true,
            terms_accepted_at: attacker.base.terms_accepted_at,
        },
    )
    .await;

    let owner_addr = serve_as(&state, owner).await;

    let spec = serde_json::to_value(crate::server::openapi::full_spec()).unwrap();
    let schemas = &spec["components"]["schemas"];
    let client = reqwest::Client::new();

    let mut accepted = BTreeSet::new();
    let mut unprobeable = BTreeSet::new();
    let mut control_refused = BTreeSet::new();
    let mut statuses = Vec::new();
    for (method, path, operation) in operations(&spec) {
        if EXEMPT.iter().any(|(prefix, _)| path.starts_with(prefix))
            || !names_rows(&path, operation)
        {
            continue;
        }
        let Some(route_rows) = route_entity(operation)
            .and_then(|entity| rows.get(&entity))
            .filter(|rows| !rows.is_empty())
        else {
            unprobeable.insert(format!("{method} {path}"));
            continue;
        };
        let route = Route {
            method: &method,
            path: &path,
            operation,
            schemas,
            rows: route_rows,
        };

        // Reads prove the ids are good: the rows' own owner gets them, so a refusal to the
        // attacker is about the tenant, not a malformed route.
        if method == "get" {
            let owned = route.call(&client, owner_addr, target_sites[0]).await;
            let vouched = !CONTROL_EXEMPT.iter().any(|(exempt, _)| path == *exempt);
            if vouched && !owned.iter().any(|status| (200..300).contains(status)) {
                control_refused.insert(format!("{method} {path} {owned:?}"));
            }
        }

        let attacker_statuses = route.call(&client, addr, own_site.id).await;
        if attacker_statuses
            .iter()
            .any(|status| (200..300).contains(status))
        {
            accepted.insert(format!("{method} {path}"));
        }
        statuses.push(format!("{method} {path} {attacker_statuses:?}"));
    }

    println!("attacker statuses:\n{}", statuses.join("\n"));
    assert!(
        control_refused.is_empty(),
        "the rows' owner could not read these, so the probe proves nothing there:\n{control_refused:#?}"
    );
    assert!(
        accepted.is_empty() && unprobeable.is_empty(),
        "accepted another organization's ids:\n{accepted:#?}\nno seeded row to probe with:\n{unprobeable:#?}"
    );
}

/// A key holding one empty site, at the highest role, so nothing but scope refuses it.
#[tokio::test]
async fn no_route_reaches_a_row_on_a_site_outside_the_callers_scope() {
    let (state, _container) = cloud_test_state().await;
    let seeded = seed_org(&state).await;
    let other_site = state
        .services
        .site_service
        .create(site(&seeded.id), AuthenticatedEntity::System)
        .await
        .unwrap();
    let in_scope = serve_as(&state, seeded.owner.clone()).await;
    let out_of_scope = serve_as(
        &state,
        AuthenticatedEntity::ApiKey {
            api_key_id: Uuid::new_v4(),
            user_id: seeded.owner.user_id().unwrap(),
            organization_id: seeded.id,
            permissions: UserOrgPermissions::Owner,
            site_ids: vec![other_site.id],
        },
    )
    .await;

    let spec = serde_json::to_value(crate::server::openapi::full_spec()).unwrap();
    let schemas = &spec["components"]["schemas"];
    let client = reqwest::Client::new();

    let mut accepted = BTreeSet::new();
    let mut control_refused = BTreeSet::new();
    let mut resources_probed = BTreeSet::new();
    for (method, path, operation) in operations(&spec) {
        if EXEMPT.iter().any(|(prefix, _)| path.starts_with(prefix))
            || !names_rows(&path, operation)
        {
            continue;
        }
        let Some(entity) = route_entity(operation) else {
            continue;
        };
        let resource = entity.resource();
        if resource.keying() != Keying::Site {
            continue;
        }
        let Some(rows) = seeded.rows.get(&entity).filter(|rows| !rows.is_empty()) else {
            continue;
        };
        let route = Route {
            method: &method,
            path: &path,
            operation,
            schemas,
            rows,
        };

        if method == "get"
            && !CONTROL_EXEMPT.iter().any(|(exempt, _)| path == *exempt)
            && !route
                .call(&client, in_scope, seeded.site_ids[0])
                .await
                .iter()
                .any(|status| (200..300).contains(status))
        {
            control_refused.insert(format!("{method} {path}"));
        }

        resources_probed.insert(resource.to_string());
        let statuses = route.call(&client, out_of_scope, other_site.id).await;
        if statuses.iter().any(|status| (200..300).contains(status)) {
            accepted.insert(format!("{method} {path} {statuses:?}"));
        }
    }

    assert!(
        control_refused.is_empty(),
        "the rows' owner could not read these, so the test proves nothing there:\n{control_refused:#?}"
    );
    assert!(
        accepted.is_empty(),
        "reached a row on a site outside the caller's scope:\n{accepted:#?}"
    );
    let site_keyed = Resource::iter().filter(|r| r.keying() == Keying::Site);
    for resource in site_keyed {
        assert!(
            resources_probed.contains(&resource.to_string()),
            "no route probed {resource}"
        );
    }
}

/// Authenticated routes that run no `Can` check, and why.
const NO_RESOURCE_CHECK: &[(&str, &str)] = &[
    (
        "get /api/v1/search",
        "with no query it returns nothing; a query asks per type, covered by `global_search`",
    ),
    (
        "get /api/v1/release-notes/unseen",
        "the caller's own read marker",
    ),
    (
        "post /api/v1/release-notes/seen",
        "the caller's own read marker",
    ),
    (
        "post /api/v1/organizations/profile",
        "the caller's own onboarding answers",
    ),
    (
        "post /api/v1/organizations/referral-source",
        "the caller's own onboarding answer",
    ),
    (
        "put /api/v1/users/{id}",
        "refuses anyone but the record's own user",
    ),
];

/// A route reached only with a daemon key. Its identity is the check, in `report_scope.rs`.
fn daemon_only(operation: &Value) -> bool {
    operation["security"].as_array().is_some_and(|schemes| {
        !schemes.is_empty()
            && schemes
                .iter()
                .all(|scheme| scheme.get("daemon_api_key").is_some())
    })
}

#[tokio::test]
async fn every_authenticated_route_records_an_authorization() {
    let (state, _container) = cloud_test_state().await;
    let seeded = seed_org(&state).await;
    let addr = serve_as(&state, seeded.owner.clone()).await;

    let spec = serde_json::to_value(crate::server::openapi::full_spec()).unwrap();
    let schemas = &spec["components"]["schemas"];
    let client = reqwest::Client::new();
    let placeholder = vec![json!({"id": Uuid::nil()})];

    let mut unrecorded = BTreeSet::new();
    for (method, path, operation) in operations(&spec) {
        let label = format!("{method} {path}");
        if PUBLIC.iter().any(|prefix| path.starts_with(prefix))
            || daemon_only(operation)
            || NO_RESOURCE_CHECK.iter().any(|(exempt, _)| label == *exempt)
        {
            continue;
        }
        let rows = route_entity(operation)
            .and_then(|entity| seeded.rows.get(&entity))
            .filter(|rows| !rows.is_empty())
            .unwrap_or(&placeholder);
        let route = Route {
            method: &method,
            path: &path,
            operation,
            schemas,
            rows,
        };
        let recorded = route
            .recorded_checks(&client, addr, seeded.site_ids[0])
            .await;
        if recorded.is_empty() {
            unrecorded.insert(label);
        }
    }

    assert!(
        unrecorded.is_empty(),
        "these routes ran no authorization check:\n{unrecorded:#?}"
    );
}

/// One operation from the spec, with the rows of the entity it acts on.
pub(super) struct Route<'a> {
    pub(super) method: &'a str,
    pub(super) path: &'a str,
    pub(super) operation: &'a Value,
    pub(super) schemas: &'a Value,
    pub(super) rows: &'a [Value],
}

impl Route<'_> {
    /// Calls the route with each body from [`bodies`], returning the statuses. A route naming two
    /// ids of one type gets two different rows, so a merge isn't refused as a merge with itself.
    pub(super) async fn call(
        &self,
        client: &reqwest::Client,
        addr: SocketAddr,
        own_site: Uuid,
    ) -> Vec<u16> {
        let mut statuses = Vec::new();
        for request in self.requests(client, addr, own_site) {
            statuses.push(match request.send().await {
                Ok(response) => response.status().as_u16(),
                Err(_) => 0,
            });
        }
        statuses
    }

    /// The checks the route recorded across its requests.
    async fn recorded_checks(
        &self,
        client: &reqwest::Client,
        addr: SocketAddr,
        own_site: Uuid,
    ) -> String {
        let mut recorded = String::new();
        for request in self.requests(client, addr, own_site) {
            if let Ok(response) = request.send().await
                && let Some(checks) = response.headers().get(RECORDED_CHECKS_HEADER)
            {
                recorded.push_str(checks.to_str().unwrap_or_default());
            }
        }
        recorded
    }

    fn requests(
        &self,
        client: &reqwest::Client,
        addr: SocketAddr,
        own_site: Uuid,
    ) -> Vec<reqwest::RequestBuilder> {
        let row = &self.rows[0];
        let other = self.rows.get(1).unwrap_or(row);
        let params: Vec<&Value> = self.operation["parameters"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();

        let mut url = self.path.to_string();
        let mut ids = [row, other].into_iter().cycle();
        for param in params.iter().filter(|p| p["in"] == "path") {
            let name = param["name"].as_str().unwrap_or_default();
            // A path param that isn't a uuid (a field name, a view) takes its schema's first value.
            let value = if param["schema"]["format"] == "uuid" {
                ids.next().unwrap()["id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            } else {
                let value = sample(self.schemas, &param["schema"], Uuid::nil(), own_site, 0);
                value.as_str().unwrap_or("x").to_string()
            };
            url = url.replace(&format!("{{{name}}}"), &value);
        }
        let query: Vec<(String, String)> = params
            .iter()
            .filter(|p| p["in"] == "query" && p["required"] == true)
            .filter_map(|p| p["name"].as_str())
            .map(|name| (name.to_string(), own_site.to_string()))
            .collect();

        bodies(self.schemas, self.operation, row, other, own_site)
            .into_iter()
            .map(|body| {
                let verb =
                    reqwest::Method::from_bytes(self.method.to_uppercase().as_bytes()).unwrap();
                let request = client
                    .request(verb, format!("http://{addr}{url}"))
                    .query(&query);
                match &body {
                    Some(body) => request.json(body),
                    None => request,
                }
            })
            .collect()
    }
}
