//! Reading a Stripe webhook delivery without depending on its API version.
//!
//! Stripe renders each delivery in the receiving endpoint's API version, which
//! need not be the one our typed client is built against. Parsing the event's
//! object with those types fails whenever a version adds a required field,
//! as `2026-08-26.dahlia` did with `Subscription.billing_schedules`. So the
//! delivery is treated as a notification: the signature is checked over the
//! raw body, only the envelope's long-stable fields are read, and the object
//! itself is fetched from the API in our own version.
//!
//! async-stripe checks the signature only as part of parsing the whole event,
//! so the check is done here, following Stripe's documented manual
//! verification.
use anyhow::{Error, anyhow};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use serde_json::Value;
use sha2::Sha256;
use std::str::FromStr;
use stripe_core::EventType;

/// Oldest signature timestamp accepted, in seconds. Stripe's libraries default
/// to the same, and it bounds how long a captured delivery can be replayed.
const SIGNATURE_TOLERANCE_SECS: i64 = 300;

/// Check `header` (the `Stripe-Signature` value) against the raw `payload`.
///
/// The header carries a timestamp `t` and one `v1` signature per active
/// signing secret: HMAC-SHA256 of `"{t}.{payload}"`. Any `v1` matching ours
/// passes. Other schemes are ignored, as Stripe asks, to rule out downgrades.
pub fn verify_signature(payload: &str, header: &str, secret: &str, now: i64) -> Result<(), Error> {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        match part.trim().split_once('=') {
            Some(("t", value)) => timestamp = value.parse::<i64>().ok(),
            Some(("v1", value)) => signatures.push(value),
            _ => {}
        }
    }
    let timestamp = timestamp.ok_or_else(|| anyhow!("Stripe signature has no timestamp"))?;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| anyhow!("Stripe webhook secret is not a usable key"))?;
    mac.update(timestamp.to_string().as_bytes());
    mac.update(b".");
    mac.update(payload.as_bytes());

    // `verify_slice` compares in constant time.
    let matched = signatures.iter().any(|signature| {
        hex::decode(signature).is_ok_and(|bytes| mac.clone().verify_slice(&bytes).is_ok())
    });
    if !matched {
        return Err(anyhow!("Stripe webhook signature does not match"));
    }
    if (now - timestamp).abs() > SIGNATURE_TOLERANCE_SECS {
        return Err(anyhow!("Stripe webhook signature is too old"));
    }
    Ok(())
}

/// The fields of a delivery the webhook path reads. Everything else about the
/// object comes from fetching it.
#[derive(Debug)]
pub struct StripeEnvelope {
    pub id: String,
    pub type_: EventType,
    /// The version the delivery was rendered in, for logs.
    pub api_version: Option<String>,
    pub created: i64,
    /// `data.object.id`: the subscription, invoice or payment method. Absent
    /// on a preview such as `invoice.upcoming`, whose invoice does not exist
    /// yet. Handlers that need it read it through [`Self::object_id`].
    pub object_id: Option<String>,
    /// `data.object.customer`, where the object has one.
    pub customer: Option<String>,
    /// `data.previous_attributes.customer`: who a detached payment method
    /// belonged to, since the object no longer says.
    pub previous_customer: Option<String>,
    /// Whether the subscription as this event recorded it carried the
    /// customer's cancellation feedback or comment. Stripe Portal sends the
    /// feedback in a second update after the cancellation itself, and a fetch
    /// made for the first update can already see it, so which event the
    /// feedback arrived with is read from the event, not the fetch.
    pub carries_cancellation_feedback: bool,
}

#[derive(Deserialize)]
struct RawEnvelope {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    api_version: Option<String>,
    created: i64,
    data: RawData,
}

#[derive(Deserialize)]
struct RawData {
    object: RawObject,
    previous_attributes: Option<Value>,
}

#[derive(Deserialize)]
struct RawObject {
    id: Option<String>,
    customer: Option<Value>,
    cancellation_details: Option<Value>,
}

impl StripeEnvelope {
    pub fn parse(payload: &str) -> Result<Self, Error> {
        let raw: RawEnvelope = serde_json::from_str(payload)
            .map_err(|e| anyhow!("Stripe webhook envelope did not parse: {e}"))?;
        let object = raw.data.object;

        let carries_cancellation_feedback =
            object.cancellation_details.as_ref().is_some_and(|details| {
                ["feedback", "comment"]
                    .iter()
                    .any(|key| details.get(key).is_some_and(|value| !value.is_null()))
            });

        Ok(Self {
            type_: EventType::from_str(&raw.type_).unwrap_or_else(|never| match never {}),
            id: raw.id,
            api_version: raw.api_version,
            created: raw.created,
            object_id: object.id,
            customer: object.customer.as_ref().and_then(id_of),
            previous_customer: raw
                .data
                .previous_attributes
                .as_ref()
                .and_then(|previous| previous.get("customer"))
                .and_then(id_of),
            carries_cancellation_feedback,
        })
    }

    /// The object id a handler acts on. Every handled event type carries one;
    /// only previews of objects that don't exist yet leave it out.
    pub fn object_id(&self) -> Result<&str, Error> {
        self.object_id.as_deref().ok_or_else(|| {
            anyhow!(
                "Stripe event {} ({:?}) has no data.object.id",
                self.id,
                self.type_
            )
        })
    }
}

/// An id field, whether Stripe sent the bare id or the expanded object.
fn id_of(value: &Value) -> Option<String> {
    value
        .as_str()
        .or_else(|| value.get("id").and_then(Value::as_str))
        .map(str::to_string)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) const SUBSCRIPTION_CREATED_CLOVER: &str =
        include_str!("fixtures/customer_subscription_created.clover.json");
    pub(crate) const SUBSCRIPTION_CREATED_DAHLIA: &str =
        include_str!("fixtures/customer_subscription_created.dahlia.json");
    pub(crate) const INVOICE_PAID_CLOVER: &str = include_str!("fixtures/invoice_paid.clover.json");
    pub(crate) const INVOICE_PAID_DAHLIA: &str = include_str!("fixtures/invoice_paid.dahlia.json");
    const PAYMENT_METHOD_ATTACHED_CLOVER: &str =
        include_str!("fixtures/payment_method_attached.clover.json");
    const PAYMENT_METHOD_ATTACHED_DAHLIA: &str =
        include_str!("fixtures/payment_method_attached.dahlia.json");
    /// `evt_1ULymsAkcpHEg5IA9qAMhzjj`: its invoice is a preview with no `id`.
    const INVOICE_UPCOMING_CLOVER: &str = include_str!("fixtures/invoice_upcoming.clover.json");

    const SECRET: &str = "whsec_test_secret";
    const NOW: i64 = 1_790_820_700;

    pub(crate) fn header(payload: &str, secret: &str, timestamp: i64) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(format!("{timestamp}.{payload}").as_bytes());
        let v1 = hex::encode(mac.finalize().into_bytes());
        format!("t={timestamp},v1={v1}")
    }

    /// The regression case is `evt_1ULZRvAkcpHEg5IAyzQrl0Oa`, a clover
    /// delivery that the dahlia-built SDK could not parse. Both renderings of
    /// each event read the same, since the envelope does not depend on the
    /// version.
    #[test]
    fn clover_and_dahlia_deliveries_read_the_same() {
        let cases = [
            (
                SUBSCRIPTION_CREATED_CLOVER,
                SUBSCRIPTION_CREATED_DAHLIA,
                EventType::CustomerSubscriptionCreated,
            ),
            (
                INVOICE_PAID_CLOVER,
                INVOICE_PAID_DAHLIA,
                EventType::InvoicePaid,
            ),
            (
                PAYMENT_METHOD_ATTACHED_CLOVER,
                PAYMENT_METHOD_ATTACHED_DAHLIA,
                EventType::PaymentMethodAttached,
            ),
        ];
        for (clover, dahlia, type_) in cases {
            let clover = StripeEnvelope::parse(clover).unwrap();
            let dahlia = StripeEnvelope::parse(dahlia).unwrap();
            assert_eq!(clover.type_, type_);
            assert_eq!(dahlia.type_, type_);
            assert_eq!(clover.id, dahlia.id);
            assert_eq!(clover.object_id, dahlia.object_id);
            assert_eq!(clover.customer, dahlia.customer);
            assert!(clover.customer.is_some());
            assert_ne!(clover.api_version, dahlia.api_version);
        }
    }

    /// An event whose object has no id used to fail to parse, so the endpoint
    /// answered 500 to a type it doesn't even handle and Stripe kept retrying.
    #[test]
    fn an_event_without_an_object_id_still_parses() {
        let envelope = StripeEnvelope::parse(INVOICE_UPCOMING_CLOVER).unwrap();
        assert!(envelope.object_id.is_none());
        assert!(envelope.object_id().is_err());
        assert!(envelope.customer.is_some());
    }

    #[test]
    fn an_unknown_event_type_still_parses() {
        let payload = SUBSCRIPTION_CREATED_CLOVER
            .replace("customer.subscription.created", "some.future.event");
        let envelope = StripeEnvelope::parse(&payload).unwrap();
        assert!(matches!(envelope.type_, EventType::Unknown(_)));
    }

    #[test]
    fn a_detached_payment_method_names_its_former_customer() {
        let payload = r#"{
            "id": "evt_1", "type": "payment_method.detached", "created": 1,
            "api_version": "2025-11-17.clover",
            "data": {
                "object": {"id": "pm_1", "customer": null},
                "previous_attributes": {"customer": "cus_former"}
            }
        }"#;
        let envelope = StripeEnvelope::parse(payload).unwrap();
        assert_eq!(envelope.customer, None);
        assert_eq!(envelope.previous_customer.as_deref(), Some("cus_former"));
    }

    /// Portal's first update records only Stripe's own `reason`; its second
    /// carries what the customer wrote.
    #[test]
    fn cancellation_feedback_is_read_from_the_event() {
        let with = |details: &str| {
            format!(
                r#"{{"id": "evt_1", "type": "customer.subscription.updated", "created": 1,
                    "data": {{"object": {{"id": "sub_1", "cancellation_details": {details}}}}}}}"#
            )
        };
        let reason_only =
            with(r#"{"reason": "cancellation_requested", "feedback": null, "comment": null}"#);
        let feedback = with(r#"{"reason": null, "feedback": "too_expensive", "comment": null}"#);
        let comment = with(r#"{"reason": null, "feedback": null, "comment": "pricey"}"#);

        assert!(
            !StripeEnvelope::parse(&reason_only)
                .unwrap()
                .carries_cancellation_feedback
        );
        assert!(
            StripeEnvelope::parse(&feedback)
                .unwrap()
                .carries_cancellation_feedback
        );
        assert!(
            StripeEnvelope::parse(&comment)
                .unwrap()
                .carries_cancellation_feedback
        );
    }

    #[test]
    fn a_signature_from_the_secret_passes() {
        let payload = SUBSCRIPTION_CREATED_CLOVER;
        assert!(verify_signature(payload, &header(payload, SECRET, NOW), SECRET, NOW).is_ok());
    }

    /// While a secret is being rolled, Stripe signs with both.
    #[test]
    fn any_matching_v1_signature_passes() {
        let payload = SUBSCRIPTION_CREATED_CLOVER;
        let old = header(payload, "whsec_old", NOW);
        let current = header(payload, SECRET, NOW);
        let current_v1 = current.split_once(",v1=").unwrap().1;
        let both = format!("{old},v1={current_v1}");
        assert!(verify_signature(payload, &both, SECRET, NOW).is_ok());
    }

    #[test]
    fn a_forged_or_stale_delivery_is_refused() {
        let payload = SUBSCRIPTION_CREATED_CLOVER;
        let signed = header(payload, SECRET, NOW);

        let tampered = payload.replace("trialing", "active");
        assert!(verify_signature(&tampered, &signed, SECRET, NOW).is_err());
        assert!(verify_signature(payload, &signed, "whsec_other", NOW).is_err());
        assert!(
            verify_signature(payload, &signed, SECRET, NOW + SIGNATURE_TOLERANCE_SECS + 1).is_err()
        );
        // Only v1 counts: a matching signature under another scheme does not.
        let v0 = signed.replace("v1=", "v0=");
        assert!(verify_signature(payload, &v0, SECRET, NOW).is_err());
        assert!(verify_signature(payload, "garbage", SECRET, NOW).is_err());
    }
}
