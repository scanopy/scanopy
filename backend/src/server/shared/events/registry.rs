//! Subscriber registry — `inventory`-based collection of `Subscriber<Op>` impls.
//!
//! Each `impl Subscriber<Op> for X` block in the codebase pairs with a one-line
//! `inventory::submit!(SubscriberRegistration::new::<X, Op>())` next to it. At
//! startup, `register_all_subscribers` iterates every entry, matches it to the
//! live service via `TypeId`, and registers it on the appropriate channel.
//!
//! Mirrors the existing `ServiceDefinitionFactory` pattern in
//! `services/definitions/mod.rs`.

use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::{Result, anyhow};

use crate::server::shared::events::{
    bus::{BusChannel, EventBus},
    traits::{Operation, Subscriber},
};

/// Type-erased registration function used by `SubscriberRegistration`. Each
/// (Service, Op) monomorphization gets its own `fn` pointer of this type via
/// `register_one::<S, Op>`.
type RegisterFn = fn(
    Arc<dyn Any + Send + Sync>,
    Arc<EventBus>,
    &'static str,
) -> Pin<Box<dyn Future<Output = ()> + Send>>;

/// Static registration entry submitted from each `impl Subscriber<Op>` block.
/// All fields are function pointers so the struct is `const`-constructible
/// and can land in a static via `inventory::submit!`.
pub struct SubscriberRegistration {
    /// Returns the `TypeId` of the service this entry registers. Used by the
    /// registry runner to match the entry to the live service instance.
    pub service_type: fn() -> TypeId,

    /// Computes the stable identifier `<service>:<op>` snake-cased. Called
    /// once per entry at startup.
    pub name: fn() -> &'static str,

    /// Erased registration function. Caller passes `Arc<dyn Any>` of the live
    /// service, the bus, and the resolved name; the function downcasts and
    /// calls `bus.register::<Op>(svc, name).await`.
    pub register: RegisterFn,
}

impl SubscriberRegistration {
    pub const fn new<S, Op>() -> Self
    where
        S: Subscriber<Op> + Send + Sync + 'static,
        Op: Operation + 'static,
        EventBus: BusChannel<Op>,
    {
        Self {
            service_type: TypeId::of::<S>,
            name: auto_name::<S, Op>,
            register: register_one::<S, Op>,
        }
    }
}

inventory::collect!(SubscriberRegistration);

/// The actual registration function — monomorphized per (S, Op). Stored as a
/// `fn` pointer in `SubscriberRegistration::register` so the entry is
/// `const`-constructible.
fn register_one<S, Op>(
    any: Arc<dyn Any + Send + Sync>,
    bus: Arc<EventBus>,
    name: &'static str,
) -> Pin<Box<dyn Future<Output = ()> + Send>>
where
    S: Subscriber<Op> + Send + Sync + 'static,
    Op: Operation + 'static,
    EventBus: BusChannel<Op>,
{
    Box::pin(async move {
        if let Ok(svc) = any.downcast::<S>() {
            bus.register::<Op>(svc, name).await;
        }
    })
}

/// Output of `ServiceCollector::build()` — passed to `register_all_subscribers`.
/// Carries the constructed services plus the `TypeId`s of any optional services
/// that were declared via `with_optional` but were `None`. The registry uses
/// the latter to distinguish "subscriber's service is intentionally absent"
/// (debug log + skip) from "subscriber registered but its service was never
/// added to the factory" (real bug — fail fast at startup).
pub struct CollectedServices {
    pub services: Vec<Arc<dyn Any + Send + Sync>>,
    pub optional_absent: HashSet<TypeId>,
}

/// Builder for `CollectedServices`. Hides the `Arc<S> -> Arc<dyn Any>`
/// coercion so the factory's `all_services` method reads as a flat
/// `.with(...)` chain.
pub struct ServiceCollector {
    services: Vec<Arc<dyn Any + Send + Sync>>,
    optional_absent: HashSet<TypeId>,
}

impl ServiceCollector {
    pub fn new() -> Self {
        Self {
            services: Vec::new(),
            optional_absent: HashSet::new(),
        }
    }

    pub fn with<S: Any + Send + Sync + 'static>(mut self, s: Arc<S>) -> Self {
        self.services.push(s);
        self
    }

    pub fn with_optional<S: Any + Send + Sync + 'static>(mut self, s: Option<Arc<S>>) -> Self {
        match s {
            Some(svc) => self.services.push(svc),
            None => {
                self.optional_absent.insert(TypeId::of::<S>());
            }
        }
        self
    }

    pub fn build(self) -> CollectedServices {
        CollectedServices {
            services: self.services,
            optional_absent: self.optional_absent,
        }
    }
}

impl Default for ServiceCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Walk the registry once at startup, registering every entry whose
/// `service_type` matches a service in `services`. Fails fast on:
/// - duplicate subscriber names (would mean two distinct services share a
///   `type_name` last segment after snake-casing — a real bug worth catching)
/// - inventory entries with no matching service in the Vec (almost certainly
///   means a new service was added to `ServiceFactory` but its corresponding
///   subscribers aren't reachable because it isn't in `all_services`)
pub async fn register_all_subscribers(
    services: CollectedServices,
    bus: Arc<EventBus>,
) -> Result<()> {
    let mut seen: HashSet<&'static str> = HashSet::new();
    for entry in inventory::iter::<SubscriberRegistration> {
        let name = (entry.name)();
        if !seen.insert(name) {
            return Err(anyhow!(
                "duplicate subscriber name: {}. Each (Service, Op) pair must yield a unique \
                 auto-generated name; this indicates two services share a snake_case last \
                 segment of their type name.",
                name,
            ));
        }
        let target = (entry.service_type)();
        let mut matched = false;
        for svc in &services.services {
            if (**svc).type_id() == target {
                (entry.register)(svc.clone(), bus.clone(), name).await;
                matched = true;
                break;
            }
        }
        if !matched {
            if services.optional_absent.contains(&target) {
                tracing::debug!(
                    subscriber = name,
                    "optional service not constructed; skipping subscriber",
                );
                continue;
            }
            return Err(anyhow!(
                "subscriber '{}' registered via inventory but no matching service in \
                 ServiceFactory::all_services() — was a new service added to the factory \
                 without updating that method?",
                name,
            ));
        }
    }
    Ok(())
}

/// Computes `<service_snake>:<op_snake>` from `type_name`. Each (S, Op)
/// monomorphization gets one leaked string on first call; the leak is bounded
/// by the number of distinct (S, Op) pairs (~14 in current codebase) and lasts
/// the life of the program.
fn auto_name<S: 'static, Op: 'static>() -> &'static str {
    let s = snake_case_last_segment(std::any::type_name::<S>());
    let o = snake_case_last_segment(std::any::type_name::<Op>());
    let op = o.replace("_operation", "");
    Box::leak(format!("{}:{}", s, op).into_boxed_str())
}

/// `crate::path::ServiceName` -> `service_name`. Generic-parameter brackets
/// are stripped before snake-casing.
fn snake_case_last_segment(t: &str) -> String {
    let no_generics = t.split('<').next().unwrap_or(t);
    let last = no_generics.rsplit("::").next().unwrap_or(no_generics);
    to_snake_case(last)
}

fn to_snake_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `scanopy_event_subscriber_errors_total` splits each name at `:` into its `subscriber` and
    /// `event_type` labels. Every registered subscriber must yield both halves, or its failures
    /// land under a malformed series.
    #[test]
    fn every_subscriber_name_splits_into_metric_labels() {
        let mut count = 0;
        for entry in inventory::iter::<SubscriberRegistration> {
            let name = (entry.name)();
            let (subscriber, event_type) = name
                .split_once(':')
                .unwrap_or_else(|| panic!("{name} has no ':'"));
            assert!(
                !subscriber.is_empty() && !event_type.is_empty() && !event_type.contains(':'),
                "{name} does not split into subscriber and event_type"
            );
            count += 1;
        }
        assert!(count > 0, "no subscribers registered");
    }
}
