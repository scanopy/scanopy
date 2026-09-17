//! Paying for a self-hosted license by invoice: billing details, sent
//! invoices, quotes, and the purchase order number printed on them.
use super::*;
use crate::server::billing::types::api::{
    InvoiceBillingDetails, InvoiceBillingMode, InvoiceBillingStatus, OpenInvoice, PendingQuote,
};
use crate::server::billing::types::base::InvoiceCollection;
use crate::server::shared::types::api::ValidationError;
use stripe_billing::invoice::{FinalizeInvoiceInvoice, ListInvoice};
use stripe_billing::quote::{
    AcceptQuote, CancelQuote, CreateQuote, CreateQuoteInvoiceSettings, CreateQuoteLineItems,
    CreateQuoteSubscriptionData, FinalizeQuoteQuote, ListQuote,
};
use stripe_billing::subscription::{
    UpdateSubscriptionTrialSettings, UpdateSubscriptionTrialSettingsEndBehavior,
    UpdateSubscriptionTrialSettingsEndBehaviorMissingPaymentMethod,
};
use stripe_billing::tax_id::{CreateCustomerTaxId, CreateCustomerTaxIdType, ListCustomerTaxId};
use stripe_billing::{Quote, QuoteCollectionMethod, QuoteStatus};
use stripe_core::customer::{
    CustomFieldParams, OptionalFieldsCustomerAddress, RetrieveCustomer, RetrieveCustomerReturned,
};
use stripe_shared::{InvoiceStatus, SubscriptionCollectionMethod};

/// Days a buyer has to pay a sent invoice.
pub const INVOICE_DAYS_UNTIL_DUE: u32 = 30;

/// Days a quote stays open for the buyer's procurement to raise a PO.
const QUOTE_VALID_DAYS: i64 = 30;

/// Name of the invoice custom field carrying the buyer's purchase order.
const PO_NUMBER_FIELD: &str = "PO Number";

/// Stripe serves rendered quote PDFs from this host, not the API host.
const STRIPE_FILES_BASE: &str = "https://files.stripe.com/v1";

impl BillingService {
    /// Record the billing entity on the Stripe customer and switch the org to
    /// paying by invoice: either issue the first invoice now or open a quote.
    ///
    /// With a live subscription (a trial, or a card subscription) the plan is
    /// the org's current one. Without one, `plan` names what to invoice for.
    pub async fn set_up_invoice_billing(
        &self,
        organization_id: Uuid,
        details: InvoiceBillingDetails,
        mode: InvoiceBillingMode,
        plan: Option<BillingPlan>,
        authentication: AuthenticatedEntity,
    ) -> Result<String, Error> {
        let organization = self.get_organization(organization_id).await?;
        let current = self.find_current_subscription(&organization).await.ok();

        let plan = match (&current, plan) {
            (Some(_), _) => organization
                .base
                .plan
                .ok_or_else(|| anyhow!("Organization has no plan"))?,
            (None, Some(plan)) if self.get_plans().contains(&plan) => plan,
            (None, _) => return Err(refused("Choose a plan to invoice for")),
        };
        if plan.license_plan().is_none() {
            return Err(refused(SELF_HOSTED_ONLY));
        }

        let customer_id = self
            .get_or_create_customer(organization_id, authentication)
            .await?;
        self.apply_invoice_billing_details(&customer_id, &details)
            .await?;

        let base_price = self
            .get_price_from_lookup_key(plan.stripe_base_price_lookup_key())
            .await?
            .ok_or_else(|| anyhow!("Could not find base price for selected plan"))?;

        match mode {
            InvoiceBillingMode::Quote => {
                self.create_quote(organization_id, &customer_id, plan, &base_price)
                    .await?;
                Ok("Quote created. Download it from the License tab.".to_string())
            }
            InvoiceBillingMode::SendInvoice => {
                let subscription = match current {
                    Some(sub) => {
                        let mut update = UpdateSubscription::new(&sub.id)
                            .collection_method(SubscriptionCollectionMethod::SendInvoice)
                            .days_until_due(INVOICE_DAYS_UNTIL_DUE);
                        // A trial ends now so the buyer's finance team has an
                        // invoice to pay; the key stays valid throughout.
                        if sub.status == SubscriptionStatus::Trialing {
                            update = update
                                .trial_end(UpdateSubscriptionTrialEnd::Now)
                                .trial_settings(UpdateSubscriptionTrialSettings::new(
                                    UpdateSubscriptionTrialSettingsEndBehavior::new(
                                        UpdateSubscriptionTrialSettingsEndBehaviorMissingPaymentMethod::CreateInvoice,
                                    ),
                                ));
                        }
                        update.send(&self.stripe).await?
                    }
                    None => {
                        CreateSubscription::new(customer_id.clone())
                            .items(vec![CreateSubscriptionItems {
                                price: Some(base_price.id.to_string()),
                                quantity: Some(1),
                                ..Default::default()
                            }])
                            .collection_method(SubscriptionCollectionMethod::SendInvoice)
                            .days_until_due(INVOICE_DAYS_UNTIL_DUE)
                            .metadata(subscription_metadata(organization_id, plan)?)
                            .send(&self.stripe)
                            .await?
                    }
                };

                self.finalize_latest_invoice(&subscription).await?;

                tracing::info!(
                    organization_id = %organization_id,
                    subscription_id = %subscription.id,
                    plan = %plan.name(),
                    "Subscription switched to invoice billing"
                );
                Ok(
                    "Invoice sent. You can keep using your license key while it is paid."
                        .to_string(),
                )
            }
        }
    }

    /// Invoice billing state for the License tab: collection method, PO
    /// number, the unpaid invoice, and an open quote.
    pub async fn invoice_billing_status(
        &self,
        organization_id: Uuid,
    ) -> Result<InvoiceBillingStatus, Error> {
        let organization = self.get_organization(organization_id).await?;
        if organization
            .base
            .plan
            .is_none_or(|plan| plan.license_plan().is_none())
        {
            return Err(refused(SELF_HOSTED_ONLY));
        }
        let Some(customer_id) = organization
            .base
            .stripe_customer_id
            .clone()
            .map(CustomerId::from)
        else {
            return Ok(InvoiceBillingStatus {
                bills_by_invoice: false,
                po_number: None,
                open_invoice: None,
                pending_quote: None,
            });
        };

        let bills_by_invoice = self
            .find_current_subscription(&organization)
            .await
            .is_ok_and(|sub| sub.collection_method == SubscriptionCollectionMethod::SendInvoice);

        let open_invoice = self
            .open_license_invoices(&customer_id)
            .await?
            .into_iter()
            .next()
            .map(|invoice| OpenInvoice {
                number: invoice.number.clone(),
                amount_due_cents: invoice.amount_due,
                currency: invoice.currency.to_string(),
                due_date: invoice
                    .due_date
                    .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0)),
                hosted_invoice_url: invoice.hosted_invoice_url.clone(),
            });

        let pending_quote = self
            .open_quote(&customer_id)
            .await?
            .map(|quote| PendingQuote {
                number: quote.number.clone(),
                amount_total_cents: quote.amount_total,
                currency: quote
                    .currency
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "usd".to_string()),
                expires_at: DateTime::<Utc>::from_timestamp(quote.expires_at, 0)
                    .unwrap_or_else(Utc::now),
            });

        Ok(InvoiceBillingStatus {
            bills_by_invoice,
            po_number: self.po_number(&customer_id).await?,
            open_invoice,
            pending_quote,
        })
    }

    /// Start of the earliest license period an open (unpaid) invoice bills
    /// for, or `None` when every license invoice is paid. Air-gapped keys are
    /// capped here, since nothing can take one back once minted.
    pub async fn license_unpaid_from(
        &self,
        organization_id: Uuid,
    ) -> Result<Option<DateTime<Utc>>, Error> {
        let organization = self.get_organization(organization_id).await?;
        let Some(customer_id) = organization.base.stripe_customer_id.map(CustomerId::from) else {
            return Ok(None);
        };
        Ok(self
            .open_license_invoices(&customer_id)
            .await?
            .iter()
            .filter_map(|invoice| BillingInvoice::from(invoice).license_unpaid_from())
            .min())
    }

    /// Accept the open quote: Stripe creates the invoiced subscription from
    /// it, and the subscription webhook retires any trial subscription.
    pub async fn accept_quote(
        &self,
        organization_id: Uuid,
        po_number: Option<String>,
    ) -> Result<String, Error> {
        let customer_id = self.customer_id(organization_id).await?;
        let quote = self
            .open_quote(&customer_id)
            .await?
            .ok_or_else(|| refused("There is no open quote to accept"))?;

        if po_number.is_some() {
            self.set_po_number(&customer_id, po_number).await?;
        }

        AcceptQuote::new(quote.id.clone())
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!("Stripe rejected the quote acceptance: {e}"))?;

        tracing::info!(
            organization_id = %organization_id,
            quote_id = %quote.id,
            "Quote accepted"
        );
        Ok("Quote accepted. Your invoice is on its way.".to_string())
    }

    pub async fn cancel_quote(&self, organization_id: Uuid) -> Result<(), Error> {
        let customer_id = self.customer_id(organization_id).await?;
        let quote = self
            .open_quote(&customer_id)
            .await?
            .ok_or_else(|| refused("There is no open quote to cancel"))?;
        CancelQuote::new(quote.id).send(&self.stripe).await?;
        Ok(())
    }

    /// The open quote rendered as a PDF. Stripe serves it from its files host
    /// behind the secret key, so the browser downloads it through us.
    pub async fn quote_pdf(&self, organization_id: Uuid) -> Result<(String, Vec<u8>), Error> {
        let customer_id = self.customer_id(organization_id).await?;
        let quote = self
            .open_quote(&customer_id)
            .await?
            .ok_or_else(|| refused("There is no open quote to download"))?;

        let response = self
            .files_http
            .get(format!("{STRIPE_FILES_BASE}/quotes/{}/pdf", quote.id))
            .bearer_auth(&self.stripe_secret)
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?.to_vec();

        let filename = format!(
            "scanopy-quote-{}.pdf",
            quote.number.as_deref().unwrap_or(quote.id.as_str())
        );
        Ok((filename, bytes))
    }

    /// Replace the PO number printed on this org's future invoices.
    pub async fn update_po_number(
        &self,
        organization_id: Uuid,
        po_number: Option<String>,
    ) -> Result<(), Error> {
        let organization = self.get_organization(organization_id).await?;
        if organization
            .base
            .plan
            .is_none_or(|plan| plan.license_plan().is_none())
        {
            return Err(refused(SELF_HOSTED_ONLY));
        }
        let customer_id = self.customer_id(organization_id).await?;
        self.set_po_number(&customer_id, po_number).await
    }

    /// Webhook: a sent invoice was finalized. Licenses the org's servers until
    /// the invoice's due date plus grace.
    pub(crate) async fn handle_invoice_finalized(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let snapshot = BillingInvoice::from(&invoice);
        if snapshot.collection != InvoiceCollection::SendInvoice
            || snapshot.provisional_paid_through().is_none()
        {
            return Ok(());
        }
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for invoice.finalized — ignoring");
            return Ok(());
        };

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::InvoiceIssued { invoice: snapshot },
                AuthenticatedEntity::System,
            ))
            .await?;
        Ok(())
    }

    /// Webhook: a sent invoice was voided or marked uncollectible, so it will
    /// never be paid.
    pub(crate) async fn handle_invoice_voided(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let snapshot = BillingInvoice::from(&invoice);
        if snapshot.provisional_paid_through().is_none() {
            return Ok(());
        }
        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::debug!("No org found for voided invoice — ignoring");
            return Ok(());
        };

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::InvoiceVoided { invoice: snapshot },
                AuthenticatedEntity::System,
            ))
            .await?;
        Ok(())
    }

    /// Whether the org's current subscription is billed by sent invoice. A
    /// card detached from such an org leaves it with a way to pay.
    pub(crate) async fn bills_by_invoice(&self, organization: &Organization) -> bool {
        self.find_current_subscription(organization)
            .await
            .is_ok_and(|sub| sub.collection_method == SubscriptionCollectionMethod::SendInvoice)
    }

    async fn create_quote(
        &self,
        organization_id: Uuid,
        customer_id: &CustomerId,
        plan: BillingPlan,
        base_price: &Price,
    ) -> Result<Quote, Error> {
        // One open quote at a time: a new request replaces the old one.
        if let Some(existing) = self.open_quote(customer_id).await? {
            CancelQuote::new(existing.id).send(&self.stripe).await?;
        }

        let mut line_item = CreateQuoteLineItems::new();
        line_item.price = Some(base_price.id.to_string());
        line_item.quantity = Some(1);

        let mut invoice_settings = CreateQuoteInvoiceSettings::new();
        invoice_settings.days_until_due = Some(INVOICE_DAYS_UNTIL_DUE);

        let mut subscription_data = CreateQuoteSubscriptionData::new();
        subscription_data.metadata = Some(subscription_metadata(organization_id, plan)?);

        let draft = CreateQuote::new()
            .customer(customer_id.to_string())
            .collection_method(QuoteCollectionMethod::SendInvoice)
            .invoice_settings(invoice_settings)
            .line_items(vec![line_item])
            .subscription_data(subscription_data)
            .expires_at((Utc::now() + chrono::Duration::days(QUOTE_VALID_DAYS)).timestamp())
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!("Stripe rejected the quote: {e}"))?;

        let quote = FinalizeQuoteQuote::new(draft.id)
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!("Stripe could not finalize the quote: {e}"))?;

        tracing::info!(
            organization_id = %organization_id,
            quote_id = %quote.id,
            plan = %plan.name(),
            "Quote created"
        );
        Ok(quote)
    }

    async fn open_quote(&self, customer_id: &CustomerId) -> Result<Option<Quote>, Error> {
        Ok(ListQuote::new()
            .customer(customer_id.to_string())
            .status(QuoteStatus::Open)
            .limit(1)
            .send(&self.stripe)
            .await?
            .data
            .into_iter()
            .next())
    }

    /// Open invoices carrying a self-hosted license line, newest first.
    async fn open_license_invoices(
        &self,
        customer_id: &CustomerId,
    ) -> Result<Vec<stripe_billing::Invoice>, Error> {
        Ok(ListInvoice::new()
            .customer(customer_id.to_string())
            .status(InvoiceStatus::Open)
            .send(&self.stripe)
            .await?
            .data
            .into_iter()
            .filter(|invoice| {
                BillingInvoice::from(invoice)
                    .license_unpaid_from()
                    .is_some()
            })
            .collect())
    }

    /// Finalize the subscription's draft invoice now instead of after
    /// Stripe's hour-long draft window, so the buyer receives it immediately.
    /// Finalizing a sent invoice with auto-advance on emails it.
    async fn finalize_latest_invoice(&self, subscription: &Subscription) -> Result<(), Error> {
        let drafts = ListInvoice::new()
            .subscription(subscription.id.to_string())
            .status(InvoiceStatus::Draft)
            .send(&self.stripe)
            .await?;
        for id in drafts.data.into_iter().filter_map(|invoice| invoice.id) {
            FinalizeInvoiceInvoice::new(id)
                .auto_advance(true)
                .send(&self.stripe)
                .await?;
        }
        Ok(())
    }

    async fn apply_invoice_billing_details(
        &self,
        customer_id: &CustomerId,
        details: &InvoiceBillingDetails,
    ) -> Result<(), Error> {
        let mut address = OptionalFieldsCustomerAddress::new();
        address.line1 = Some(details.address.line1.clone());
        address.line2 = details.address.line2.clone();
        address.city = Some(details.address.city.clone());
        address.state = details.address.state.clone();
        address.postal_code = Some(details.address.postal_code.clone());
        address.country = Some(details.address.country.clone());

        let mut invoice_settings = UpdateCustomerInvoiceSettings::new();
        invoice_settings.custom_fields = Some(
            self.custom_fields_with_po(customer_id, details.po_number.clone())
                .await?,
        );

        UpdateCustomer::new(customer_id.clone())
            .name(details.entity_name.clone())
            .email(details.billing_email.clone())
            .address(address)
            .invoice_settings(invoice_settings)
            .send(&self.stripe)
            .await?;

        if let Some(tax_id) = &details.tax_id {
            let existing = ListCustomerTaxId::new(customer_id.clone())
                .send(&self.stripe)
                .await?;
            if !existing.data.iter().any(|t| t.value == tax_id.value) {
                let type_: CreateCustomerTaxIdType =
                    tax_id.tax_id_type.parse().unwrap_or_else(|e| match e {});
                CreateCustomerTaxId::new(customer_id.clone(), type_, tax_id.value.clone())
                    .send(&self.stripe)
                    .await
                    .map_err(|e| refused(format!("Stripe rejected the tax ID: {e}")))?;
            }
        }
        Ok(())
    }

    async fn set_po_number(
        &self,
        customer_id: &CustomerId,
        po_number: Option<String>,
    ) -> Result<(), Error> {
        let mut invoice_settings = UpdateCustomerInvoiceSettings::new();
        invoice_settings.custom_fields =
            Some(self.custom_fields_with_po(customer_id, po_number).await?);
        UpdateCustomer::new(customer_id.clone())
            .invoice_settings(invoice_settings)
            .send(&self.stripe)
            .await?;
        Ok(())
    }

    /// The customer's invoice custom fields with the PO field replaced (or
    /// removed when `po_number` is empty). Other fields are kept.
    async fn custom_fields_with_po(
        &self,
        customer_id: &CustomerId,
        po_number: Option<String>,
    ) -> Result<Vec<CustomFieldParams>, Error> {
        let mut fields: Vec<CustomFieldParams> = self
            .customer_custom_fields(customer_id)
            .await?
            .into_iter()
            .filter(|field| field.name != PO_NUMBER_FIELD)
            .map(|field| CustomFieldParams::new(field.name, field.value))
            .collect();
        if let Some(po) = po_number.map(|po| po.trim().to_string())
            && !po.is_empty()
        {
            fields.push(CustomFieldParams::new(PO_NUMBER_FIELD, po));
        }
        Ok(fields)
    }

    async fn po_number(&self, customer_id: &CustomerId) -> Result<Option<String>, Error> {
        Ok(self
            .customer_custom_fields(customer_id)
            .await?
            .into_iter()
            .find(|field| field.name == PO_NUMBER_FIELD)
            .map(|field| field.value))
    }

    async fn customer_custom_fields(
        &self,
        customer_id: &CustomerId,
    ) -> Result<Vec<stripe_shared::InvoiceSettingCustomField>, Error> {
        match RetrieveCustomer::new(customer_id.clone())
            .send(&self.stripe)
            .await?
        {
            RetrieveCustomerReturned::Customer(customer) => Ok(customer
                .invoice_settings
                .and_then(|settings| settings.custom_fields)
                .unwrap_or_default()),
            RetrieveCustomerReturned::DeletedCustomer(_) => Ok(Vec::new()),
        }
    }

    async fn customer_id(&self, organization_id: Uuid) -> Result<CustomerId, Error> {
        self.get_organization(organization_id)
            .await?
            .base
            .stripe_customer_id
            .map(CustomerId::from)
            .ok_or_else(|| refused("Organization has no billing account"))
    }
}

const SELF_HOSTED_ONLY: &str = "Invoice billing is available on self-hosted plans only";

/// A request the buyer can correct, returned as 400 rather than 500.
fn refused(message: impl Into<String>) -> Error {
    ValidationError::new(message).into()
}

/// Subscription metadata identifying the org and plan, as every Stripe
/// subscription the webhooks read carries.
fn subscription_metadata(
    organization_id: Uuid,
    plan: BillingPlan,
) -> Result<std::collections::HashMap<String, String>, Error> {
    Ok(StripeSubscriptionMetadata {
        organization_id: Some(organization_id),
        plan: Some(plan),
        ..Default::default()
    }
    .to_stripe())
}
