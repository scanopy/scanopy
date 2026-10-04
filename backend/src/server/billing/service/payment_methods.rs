//! The payment methods Scanopy offers, held in one named Payment Method
//! Configuration that every payment surface Scanopy creates points at.
//!
//! Without it Stripe falls back to the account's default configuration, and a
//! SetupIntent carries no currency for Stripe to filter on: BLIK (PLN only)
//! was offered in the payment modal though every Scanopy price is USD and a
//! saved BLIK mandate can only be charged in PLN. The configuration is defined
//! here and reconciled at startup, so test and live mode hold the same set and
//! a Dashboard edit does not outlive the next start.
use super::*;
use std::collections::BTreeMap;
use stripe_payment::PaymentMethodConfiguration;
use stripe_payment::PaymentMethodConfigurationId;
use stripe_payment::payment_method_configuration::{
    CreatePaymentMethodConfiguration, ListPaymentMethodConfiguration,
};

/// Name of the configuration in the Dashboard, under Settings → Payment
/// methods. The customer portal and the invoice template are pointed at it
/// there.
const SUBSCRIPTION_PMC_NAME: &str = "Scanopy subscriptions";

/// The methods a customer can save to pay a subscription. Each can be saved
/// off-session without a redirect and charged later in USD. Keys are the
/// configuration's field names; every other method is turned off.
const SUBSCRIPTION_PAYMENT_METHODS: &[&str] =
    &["card", "apple_pay", "google_pay", "link", "us_bank_account"];

const ON: &str = "on";
const OFF: &str = "off";

/// Each method's current `display_preference.preference` ("on", "off" or
/// "none"), keyed by field name. Read from the serialized configuration so
/// every method Stripe returns is covered, not a list kept by hand.
fn method_preferences(
    config: &PaymentMethodConfiguration,
) -> Result<BTreeMap<String, String>, Error> {
    let serde_json::Value::Object(fields) = serde_json::to_value(config)? else {
        return Err(anyhow!(
            "Payment method configuration did not serialize to an object"
        ));
    };

    Ok(fields
        .into_iter()
        .filter_map(|(method, value)| {
            let preference = value
                .get("display_preference")?
                .get("preference")?
                .as_str()?
                .to_string();
            Some((method, preference))
        })
        .collect())
}

/// The preference each method must change to so the configuration offers
/// exactly [`SUBSCRIPTION_PAYMENT_METHODS`]. A method left at "none" follows
/// Stripe's default, which can change, so it is set explicitly.
fn preference_changes(current: &BTreeMap<String, String>) -> BTreeMap<String, &'static str> {
    current
        .iter()
        .filter_map(|(method, preference)| {
            let wanted = if SUBSCRIPTION_PAYMENT_METHODS.contains(&method.as_str()) {
                ON
            } else {
                OFF
            };
            (preference != wanted).then(|| (method.clone(), wanted))
        })
        .collect()
}

#[derive(serde::Serialize)]
struct DisplayPreferenceForm {
    preference: &'static str,
}

#[derive(serde::Serialize)]
struct MethodForm {
    display_preference: DisplayPreferenceForm,
}

#[derive(serde::Serialize)]
struct UpdateConfigurationForm {
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
    #[serde(flatten)]
    methods: BTreeMap<String, MethodForm>,
}

/// Sets method preferences by field name. The SDK's
/// `UpdatePaymentMethodConfiguration` has one typed setter per method, which
/// can't take the set computed from the configuration Stripe returned.
struct UpdateConfiguration {
    id: PaymentMethodConfigurationId,
    body: UpdateConfigurationForm,
}

impl UpdateConfiguration {
    fn new(
        id: PaymentMethodConfigurationId,
        activate: bool,
        changes: &BTreeMap<String, &'static str>,
    ) -> Self {
        Self {
            id,
            body: UpdateConfigurationForm {
                active: activate.then_some(true),
                methods: changes
                    .iter()
                    .map(|(method, preference)| {
                        (
                            method.clone(),
                            MethodForm {
                                display_preference: DisplayPreferenceForm { preference },
                            },
                        )
                    })
                    .collect(),
            },
        }
    }
}

impl StripeRequest for UpdateConfiguration {
    type Output = PaymentMethodConfiguration;

    fn build(&self) -> RequestBuilder {
        RequestBuilder::new(
            StripeMethod::Post,
            format!("/payment_method_configurations/{}", self.id),
        )
        .form(&self.body)
    }
}

impl BillingService {
    /// Find or create the subscription configuration, bring its methods in
    /// line with [`SUBSCRIPTION_PAYMENT_METHODS`], and keep its id for the
    /// SetupIntents and Checkout Sessions created after this.
    pub(crate) async fn initialize_payment_method_configuration(&self) -> Result<(), Error> {
        let existing = ListPaymentMethodConfiguration::new()
            .limit(100)
            .send(&self.stripe)
            .await?
            .data
            .into_iter()
            .find(|c| {
                c.name == SUBSCRIPTION_PMC_NAME
                    && !c.is_default
                    && c.parent.is_none()
                    && c.application.is_none()
            });

        let (config, created) = match existing {
            Some(config) => (config, false),
            None => (
                CreatePaymentMethodConfiguration::new()
                    .name(SUBSCRIPTION_PMC_NAME)
                    .send(&self.stripe)
                    .await?,
                true,
            ),
        };

        let current = method_preferences(&config)?;
        for method in SUBSCRIPTION_PAYMENT_METHODS {
            if !current.contains_key(*method) {
                tracing::warn!(
                    configuration_id = %config.id,
                    method,
                    "Payment method is not available on this Stripe account; it can't be offered"
                );
            }
        }

        let changes = preference_changes(&current);
        let updated = !changes.is_empty() || !config.active;
        let config = if updated {
            UpdateConfiguration::new(config.id.clone(), !config.active, &changes)
                .customize()
                .send(&self.stripe)
                .await?
        } else {
            config
        };

        tracing::info!(
            configuration_id = %config.id,
            created,
            updated,
            changed_methods = ?changes,
            "Payment method configuration ready"
        );

        let _ = self.payment_method_configuration.set(config.id.to_string());
        Ok(())
    }

    /// The subscription configuration's id, once startup has reconciled it.
    /// `None` leaves Stripe on the account's default configuration.
    pub(crate) fn payment_method_configuration_id(&self) -> Option<&str> {
        self.payment_method_configuration.get().map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> PaymentMethodConfiguration {
        serde_json::from_str(include_str!("fixtures/payment_method_configuration.json"))
            .expect("fixture parses as a PaymentMethodConfiguration")
    }

    #[test]
    fn every_allowed_method_is_a_configuration_field() {
        let current = method_preferences(&fixture()).unwrap();
        for method in SUBSCRIPTION_PAYMENT_METHODS {
            assert!(current.contains_key(*method), "{method} is not a field");
        }
    }

    #[test]
    fn drifted_configuration_turns_off_extras_and_on_missing() {
        let changes = preference_changes(&method_preferences(&fixture()).unwrap());

        assert_eq!(changes.get("blik"), Some(&OFF));
        assert_eq!(changes.get("sepa_debit"), Some(&OFF));
        assert_eq!(changes.get("link"), Some(&ON));
        // Stripe's default ("none") is replaced by an explicit choice.
        assert_eq!(changes.get("us_bank_account"), Some(&ON));
        assert!(!changes.contains_key("card"), "already on");
        assert!(!changes.contains_key("paypal"), "already off");
    }

    #[test]
    fn applying_the_changes_leaves_nothing_to_change() {
        let mut current = method_preferences(&fixture()).unwrap();
        for (method, preference) in preference_changes(&current) {
            current.insert(method, preference.to_string());
        }
        assert!(preference_changes(&current).is_empty());
    }

    #[test]
    fn update_form_uses_stripes_nested_field_names() {
        let changes = BTreeMap::from([("blik".to_string(), OFF), ("link".to_string(), ON)]);
        let body = UpdateConfiguration::new("pmc_123".parse().unwrap(), true, &changes)
            .build()
            .body
            .unwrap();
        let fields: BTreeMap<String, String> = url::form_urlencoded::parse(body.as_bytes())
            .into_owned()
            .collect();

        assert_eq!(
            fields
                .get("blik[display_preference][preference]")
                .map(String::as_str),
            Some("off")
        );
        assert_eq!(
            fields
                .get("link[display_preference][preference]")
                .map(String::as_str),
            Some("on")
        );
        assert_eq!(fields.get("active").map(String::as_str), Some("true"));
    }
}
