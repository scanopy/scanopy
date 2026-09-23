//! Paying for a self-hosted license by invoice: billing details, sent
//! invoices, quotes, and the purchase order number printed on them.
use super::*;
use crate::server::billing::types::api::{
    InvoiceBillingDetails, InvoiceBillingMode, InvoiceBillingStatus, OpenInvoice, PendingQuote,
};
use crate::server::billing::types::base::{InvoiceCollection, PO_NUMBER_FIELD};
use crate::server::shared::types::api::ValidationError;
use stripe_billing::invoice::{
    FinalizeInvoiceInvoice, ListInvoice, RetrieveInvoice, VoidInvoiceInvoice,
};
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

        let plan = plan_to_invoice(
            current.is_some(),
            organization.base.plan,
            plan,
            &self.get_plans(),
        )?;

        let customer_id = self
            .get_or_create_customer(organization_id, authentication.clone())
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
                Ok("Quote created. Download it from the Billing tab.".to_string())
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

                self.finalize_latest_invoice(subscription.id.as_str())
                    .await?;

                // A lapsed org is read-only until something says otherwise,
                // and only `CheckoutCompleted` implies Active: `InvoiceIssued`
                // cannot, because it fires on renewals too. The subscription
                // webhook publishes the same event, but waiting for it leaves
                // the buyer looking at a locked app with their invoice already
                // sent, so say it here as well. Publishing after finalizing
                // means an invoice that could not be issued never grants
                // anything, and the subscriber writes only on a difference, so
                // the webhook arriving later changes nothing.
                if organization.is_lapsed() {
                    self.event_bus
                        .publish(Event::new(
                            OrgScope { organization_id },
                            BillingOperation::CheckoutCompleted {
                                plan,
                                included_networks: plan.config().included_networks,
                                included_seats: plan.config().included_seats,
                                mrr_amount_cents: mrr_from_subscription(&subscription),
                                is_trialing: false,
                                next_renewal_at: next_renewal_from_subscription(&subscription),
                            },
                            authentication,
                        ))
                        .await?;
                }

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

        let bills_by_invoice = organization.base.bills_by_invoice;

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
    pub async fn accept_quote(&self, organization_id: Uuid) -> Result<String, Error> {
        let customer_id = self.customer_id(organization_id).await?;
        let quote = self
            .open_quote(&customer_id)
            .await?
            .ok_or_else(|| refused("There is no open quote to accept"))?;

        // The PO number the customer already carries is what the invoice
        // prints; `update_po_number` is the single writer, so accepting does
        // not offer a second place to set it.
        let accepted = AcceptQuote::new(quote.id.clone())
            .send(&self.stripe)
            .await
            .map_err(|e| anyhow!("Stripe rejected the quote acceptance: {e}"))?;

        // Stripe creates the subscription and leaves its first invoice in
        // draft for an hour. The buyer asked for an invoice, so send it now.
        // The `invoice.created` webhook catches anything raised after this
        // returns, so a slow subscription creation loses nothing.
        if let Some(subscription) = accepted.subscription.as_ref() {
            self.finalize_latest_invoice(subscription.id().as_str())
                .await?;
        }

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

        // Issuing an invoice is what licenses a buyer before the money
        // arrives, so an organization that already owes us for an overdue one
        // does not get a second term by issuing itself another.
        let owes = self
            .overdue_license_invoice(&organization, &snapshot)
            .await?;
        if let Some(owed) = &owes {
            tracing::info!(
                organization_id = %organization.id,
                invoice_id = %snapshot.stripe_invoice_id,
                unpaid_invoice_id = %owed,
                "Issuing a licence invoice without extending the licence: an earlier one is overdue"
            );
        }

        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::InvoiceIssued {
                    invoice: snapshot,
                    grants_licence: owes.is_none(),
                },
                AuthenticatedEntity::System,
            ))
            .await?;
        Ok(())
    }

    /// The id of another licence invoice that was already past its due date
    /// when `issued` was raised, if any.
    ///
    /// Merely open is not enough: a plan change leaves two invoices open for a
    /// moment, and a renewal is issued before the previous period ends. Only a
    /// due date that has passed is evidence of a default.
    ///
    /// Both timestamps come from Stripe. Comparing a Stripe due date against
    /// our own clock would be wrong even without the skew a test clock makes
    /// obvious, where Stripe's dates run months ahead of the server's.
    async fn overdue_license_invoice(
        &self,
        organization: &Organization,
        issued: &BillingInvoice,
    ) -> Result<Option<String>, Error> {
        let Some(customer_id) = organization
            .base
            .stripe_customer_id
            .clone()
            .map(CustomerId::from)
        else {
            return Ok(None);
        };
        let open: Vec<BillingInvoice> = self
            .open_license_invoices(&customer_id)
            .await?
            .iter()
            .map(BillingInvoice::from)
            .collect();
        Ok(overdue_other_invoice(
            &open,
            &issued.stripe_invoice_id,
            issued.created_at,
        ))
    }

    /// Webhook: a draft invoice exists. A sent invoice for a self-hosted
    /// licence is finalized at once rather than waiting out Stripe's hour-long
    /// draft window, since finalizing is what emails it to the buyer.
    ///
    /// This is the general rule behind the eager calls at the paths that
    /// create subscriptions: it also covers renewals, and quote acceptance,
    /// where Stripe creates the subscription itself.
    pub(crate) async fn handle_invoice_created(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let is_draft = invoice.status == Some(InvoiceStatus::Draft);
        if !should_finalize_now(is_draft, &BillingInvoice::from(&invoice)) {
            return Ok(());
        }
        let Some(id) = invoice.id.clone() else {
            return Ok(());
        };
        self.finalize_invoice(id).await
    }

    /// Webhook: Stripe could not finalize a draft invoice.
    ///
    /// Finalizing is what issues and emails a sent invoice, so a failure here
    /// is silent in every direction: no invoice reaches the buyer, no licence
    /// period is granted, and the app has already said the invoice went. A
    /// rejected tax ID is the usual cause, which is why the details are worth
    /// naming to the owner rather than only logging.
    pub(crate) async fn handle_invoice_finalization_failed(
        &self,
        invoice: stripe_billing::Invoice,
    ) -> Result<(), Error> {
        let snapshot = BillingInvoice::from(&invoice);
        if !is_sent_licence_invoice(&snapshot) {
            return Ok(());
        }
        let reason = invoice
            .last_finalization_error
            .as_ref()
            .and_then(|error| error.message.clone())
            .unwrap_or_else(|| "Stripe gave no reason".to_string());

        let Some(organization) = self.get_org_from_invoice(&invoice).await? else {
            tracing::error!(
                invoice_id = %snapshot.stripe_invoice_id,
                reason = %reason,
                "A licence invoice could not be finalized, and no org matches it"
            );
            return Ok(());
        };

        tracing::error!(
            organization_id = %organization.id,
            invoice_id = %snapshot.stripe_invoice_id,
            reason = %reason,
            "A licence invoice could not be finalized, so nothing was sent"
        );
        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::InvoiceFinalizationFailed {
                    invoice: snapshot,
                    reason,
                },
                AuthenticatedEntity::System,
            ))
            .await?;
        Ok(())
    }

    /// Move an organization to past due for an unpaid sent invoice, once.
    ///
    /// Called when the subscription goes `past_due`, which Stripe does by
    /// itself at the due date on `send_invoice` collection. No charge is
    /// attempted on such an invoice, so `invoice.payment_failed` never fires
    /// for one and this is the only notice that a licence buyer stopped
    /// paying.
    pub(crate) async fn report_invoice_overdue(
        &self,
        organization: &Organization,
        invoice: BillingInvoice,
    ) -> Result<(), Error> {
        // Going past due is a transition, not a repeatable fact, and how often
        // the subscription update arrives is Stripe's to decide: deliveries
        // are retried, and later updates carry the same status. Without this
        // guard each one sends the customer another overdue email.
        if organization.base.plan_status == Some(PlanStatus::PastDue) {
            return Ok(());
        }

        // Past due is a lesser state than lapsed: it keeps the app writable,
        // and the scheduler and daemon work handout keep serving. An org that
        // lapsed still holding an open invoice would be let back in by this,
        // since nothing voids its invoices when the subscription ends.
        if organization.is_lapsed() {
            tracing::debug!(
                organization_id = %organization.id,
                "Org has lapsed; leaving it there rather than moving it to past due"
            );
            return Ok(());
        }

        tracing::info!(
            organization_id = %organization.id,
            invoice_id = %invoice.stripe_invoice_id,
            "Sent licence invoice is overdue"
        );
        self.event_bus
            .publish(Event::new(
                OrgScope {
                    organization_id: organization.id,
                },
                BillingOperation::InvoiceOverdue { invoice },
                AuthenticatedEntity::System,
            ))
            .await?;
        Ok(())
    }

    /// The organization's oldest unpaid licence invoice, for the paths that
    /// start from a subscription rather than an invoice.
    pub(crate) async fn open_license_invoice(
        &self,
        organization: &Organization,
    ) -> Result<Option<BillingInvoice>, Error> {
        let Some(customer_id) = organization
            .base
            .stripe_customer_id
            .clone()
            .map(CustomerId::from)
        else {
            return Ok(None);
        };
        Ok(self
            .open_license_invoices(&customer_id)
            .await?
            .iter()
            .map(BillingInvoice::from)
            .find(|invoice| invoice.provisional_paid_through().is_some()))
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

    /// Void every open invoice carrying a self-hosted license line, and report
    /// whether there was one. Called before a plan change on an invoice-billed
    /// subscription: an unpaid invoice for the old plan would otherwise stand
    /// beside the new one, and neither would match the buyer's purchase order.
    /// The `invoice.voided` webhook gives back the license period each granted.
    pub(crate) async fn void_open_license_invoices(
        &self,
        organization: &Organization,
    ) -> Result<bool, Error> {
        let Some(customer_id) = organization
            .base
            .stripe_customer_id
            .clone()
            .map(CustomerId::from)
        else {
            return Ok(false);
        };

        let mut voided = false;
        for invoice in self.open_license_invoices(&customer_id).await? {
            let Some(id) = invoice.id.clone() else {
                continue;
            };
            VoidInvoiceInvoice::new(id.clone())
                .send(&self.stripe)
                .await?;
            voided = true;
            tracing::info!(
                organization_id = %organization.id,
                invoice_id = %id,
                "Voided the unpaid invoice for the plan being replaced"
            );
        }
        Ok(voided)
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
    pub(crate) async fn finalize_latest_invoice(&self, subscription_id: &str) -> Result<(), Error> {
        let drafts = ListInvoice::new()
            .subscription(subscription_id.to_string())
            .status(InvoiceStatus::Draft)
            .send(&self.stripe)
            .await?;
        for id in drafts.data.into_iter().filter_map(|invoice| invoice.id) {
            self.finalize_invoice(id).await?;
        }
        Ok(())
    }

    /// Finalize one invoice, treating "already finalized" as done.
    ///
    /// Two paths race to finalize the same draft: the eager call at each site
    /// that creates a subscription, and the `invoice.created` webhook. Either
    /// order sends the invoice exactly once, so the loser's
    /// [`ApiErrorsCode::InvoiceNotEditable`] reports the work already
    /// happened. Left unhandled it surfaced as a 500 on quote acceptance and
    /// made Stripe redeliver `invoice.created` forever.
    async fn finalize_invoice(&self, id: stripe_shared::InvoiceId) -> Result<(), Error> {
        let Err(finalize_error) = FinalizeInvoiceInvoice::new(id.clone())
            .auto_advance(true)
            .send(&self.stripe)
            .await
        else {
            tracing::info!(invoice_id = %id, "Finalized a sent licence invoice");
            return Ok(());
        };

        // Ask the invoice what state it reached rather than reading the
        // failure: Stripe leaves `code` unset on the re-finalize rejection, so
        // only its English message identifies it and that is copy, not API.
        // A draft that is no longer a draft was finalized by whoever won, and
        // this also covers Stripe auto-advancing it out from under us.
        match RetrieveInvoice::new(id.clone()).send(&self.stripe).await {
            Ok(invoice) if invoice.status != Some(InvoiceStatus::Draft) => {
                // Deliberately without the Stripe error: this is the success
                // path, the invoice is out, and dumping a rejection here read
                // as a failure to anyone watching the log.
                tracing::debug!(
                    invoice_id = %id,
                    status = ?invoice.status,
                    "Invoice was already finalized; treating as sent"
                );
                Ok(())
            }
            Ok(_) => Err(finalize_error.into()),
            Err(retrieve_error) => Err(anyhow!(
                "Invoice {id} could not be finalized ({finalize_error}) \
                 or read back ({retrieve_error})"
            )),
        }
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

/// The id of a licence invoice, other than `excluding`, whose due date had
/// already passed at `as_of`.
///
/// Merely open is not evidence of a default: a plan change leaves two invoices
/// open for a moment, and a renewal is issued before the previous period ends.
/// Only a due date that has passed says the customer stopped paying.
///
/// `as_of` is the issuing invoice's own creation time rather than the current
/// time, so every timestamp compared here comes from Stripe.
fn overdue_other_invoice(
    open: &[BillingInvoice],
    excluding: &str,
    as_of: DateTime<Utc>,
) -> Option<String> {
    open.iter()
        .filter(|invoice| invoice.stripe_invoice_id != excluding)
        .find(|invoice| invoice.due_date.is_some_and(|due| due < as_of))
        .map(|invoice| invoice.stripe_invoice_id.clone())
}

/// Whether this invoice is one we sent a self-hosted licence buyer, rather
/// than one Stripe collects automatically or a cloud invoice. The licence
/// emails speak about keys and servers, so a cloud customer must never
/// receive one.
fn is_sent_licence_invoice(invoice: &BillingInvoice) -> bool {
    invoice.collection == InvoiceCollection::SendInvoice && invoice.license_unpaid_from().is_some()
}

/// Whether a freshly created invoice should be finalized straight away: a
/// draft, sent to the customer rather than charged, for a self-hosted licence.
/// Anything else is left to Stripe, including invoices already finalized by
/// the path that created them.
fn should_finalize_now(is_draft: bool, invoice: &BillingInvoice) -> bool {
    is_draft
        && invoice.collection == InvoiceCollection::SendInvoice
        && invoice.license_unpaid_from().is_some()
}

/// Which plan an invoice covers.
///
/// A live subscription settles it: the org's own plan is what it pays for.
/// Without one the request names the plan, and when it doesn't — a reload
/// mid-flow loses it, since it travels in transient UI state — the org's own
/// licensed plan stands in rather than dead-ending the buyer.
fn plan_to_invoice(
    has_live_subscription: bool,
    org_plan: Option<BillingPlan>,
    requested: Option<BillingPlan>,
    purchasable: &[BillingPlan],
) -> Result<BillingPlan, Error> {
    let plan = match (has_live_subscription, requested) {
        (true, _) => org_plan.ok_or_else(|| anyhow!("Organization has no plan"))?,
        (false, Some(requested)) if purchasable.contains(&requested) => requested,
        (false, Some(_)) => return Err(refused("That plan cannot be invoiced")),
        (false, None) => org_plan
            .filter(|plan| plan.license_plan().is_some())
            .ok_or_else(|| refused("Choose a plan to invoice for"))?,
    };
    if plan.license_plan().is_none() {
        return Err(refused(SELF_HOSTED_ONLY));
    }
    Ok(plan)
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::billing::plans::{
        get_enterprise_plan, get_free_plan, get_self_hosted_plus_plan,
        get_self_hosted_standard_plan,
    };

    use crate::server::billing::types::base::BillingReason;

    fn licensed_invoice(collection: InvoiceCollection, licensed: bool) -> BillingInvoice {
        let now = Utc::now();
        BillingInvoice {
            stripe_invoice_id: "in_test".to_string(),
            amount_paid_cents: 0,
            currency: "usd".to_string(),
            created_at: now,
            period_start: now,
            period_end: now,
            billing_reason: BillingReason::SubscriptionCycle,
            line_items: vec![
                crate::server::billing::types::base::BillingInvoiceLineItem {
                    description: None,
                    amount_cents: 400_000,
                    period_start: now,
                    period_end: now + chrono::Duration::days(365),
                    product: Some(if licensed {
                        get_self_hosted_standard_plan().stripe_product_id()
                    } else {
                        get_enterprise_plan().stripe_product_id()
                    }),
                },
            ],
            invoice_pdf: None,
            hosted_invoice_url: None,
            collection,
            due_date: Some(now + chrono::Duration::days(30)),
            po_number: None,
            amount_due_cents: 400_000,
            total_cents: 400_000,
        }
    }

    /// Finalizing is what emails a sent invoice, and a draft otherwise sits in
    /// Stripe's auto-advance window for an hour.
    #[test]
    fn only_a_draft_sent_licence_invoice_is_finalized_on_creation() {
        let sent = licensed_invoice(InvoiceCollection::SendInvoice, true);
        assert!(should_finalize_now(true, &sent));
        // Already finalized by the path that created it.
        assert!(!should_finalize_now(false, &sent));
        // Charged automatically: Stripe collects it, nothing to send.
        assert!(!should_finalize_now(
            true,
            &licensed_invoice(InvoiceCollection::ChargeAutomatically, true)
        ));
        // A cloud invoice is none of our business here.
        assert!(!should_finalize_now(
            true,
            &licensed_invoice(InvoiceCollection::SendInvoice, false)
        ));
    }

    /// A failed finalize tells the owner their licence invoice never went out.
    /// Sending that to a cloud customer, or about an invoice Stripe charges
    /// automatically, would describe a licence they do not have.
    #[test]
    fn only_a_sent_licence_invoice_reports_a_failed_finalize() {
        assert!(is_sent_licence_invoice(&licensed_invoice(
            InvoiceCollection::SendInvoice,
            true
        )));
        assert!(!is_sent_licence_invoice(&licensed_invoice(
            InvoiceCollection::ChargeAutomatically,
            true
        )));
        assert!(!is_sent_licence_invoice(&licensed_invoice(
            InvoiceCollection::SendInvoice,
            false
        )));
    }

    /// Issuing an invoice is what licenses a buyer before the money arrives,
    /// so an org that left an earlier one overdue must not buy itself another
    /// term by issuing a second. The not-yet-due case is the one that must
    /// keep working: a renewal is issued before the previous period ends.
    #[test]
    fn only_an_overdue_earlier_invoice_withholds_the_licence() {
        // Stands for the issuing invoice's creation time, which is what the
        // caller passes: every timestamp here is one Stripe reported.
        let issued_at = Utc::now();
        let with = |id: &str, due: DateTime<Utc>| BillingInvoice {
            stripe_invoice_id: id.to_string(),
            due_date: Some(due),
            ..licensed_invoice(InvoiceCollection::SendInvoice, true)
        };

        let overdue = with("in_old", issued_at - chrono::Duration::days(1));
        let upcoming = with("in_next", issued_at + chrono::Duration::days(30));
        let being_issued = with("in_new", issued_at + chrono::Duration::days(30));

        assert_eq!(
            overdue_other_invoice(&[overdue.clone(), being_issued.clone()], "in_new", issued_at),
            Some("in_old".to_string())
        );
        // A renewal alongside the one being issued is not a default.
        assert_eq!(
            overdue_other_invoice(&[upcoming, being_issued.clone()], "in_new", issued_at),
            None
        );
        // The invoice being issued is never evidence against itself, however
        // its own due date falls.
        assert_eq!(
            overdue_other_invoice(
                &[with("in_new", issued_at - chrono::Duration::days(1))],
                "in_new",
                issued_at
            ),
            None
        );
        assert_eq!(overdue_other_invoice(&[], "in_new", issued_at), None);

        // The defect this replaced: a Stripe due date compared against our own
        // clock. Under a test clock Stripe's dates run months ahead, so an
        // invoice long overdue in Stripe's frame looked not yet due in ours
        // and the licence was granted anyway.
        let stripe_frame = issued_at + chrono::Duration::days(60);
        assert_eq!(
            overdue_other_invoice(
                &[with("in_old", stripe_frame - chrono::Duration::days(1))],
                "in_new",
                stripe_frame
            ),
            Some("in_old".to_string())
        );
    }

    fn purchasable() -> Vec<BillingPlan> {
        crate::server::billing::plans::get_purchasable_plans()
    }

    #[test]
    fn a_live_subscription_invoices_the_orgs_own_plan() {
        let plan = plan_to_invoice(
            true,
            Some(get_self_hosted_standard_plan()),
            Some(get_self_hosted_plus_plan()),
            &purchasable(),
        )
        .unwrap();
        assert_eq!(plan, get_self_hosted_standard_plan());
    }

    #[test]
    fn without_a_subscription_the_request_names_the_plan() {
        let plan = plan_to_invoice(
            false,
            None,
            Some(get_self_hosted_plus_plan()),
            &purchasable(),
        )
        .unwrap();
        assert_eq!(plan, get_self_hosted_plus_plan());
    }

    /// The plan travels in transient UI state, so a reload mid-flow arrives
    /// without one. An org already on a licensed plan is not asked again.
    #[test]
    fn a_request_with_no_plan_falls_back_to_the_orgs_licensed_plan() {
        let plan = plan_to_invoice(
            false,
            Some(get_self_hosted_standard_plan()),
            None,
            &purchasable(),
        )
        .unwrap();
        assert_eq!(plan, get_self_hosted_standard_plan());

        assert!(plan_to_invoice(false, Some(get_free_plan()), None, &purchasable()).is_err());
        assert!(plan_to_invoice(false, None, None, &purchasable()).is_err());
    }

    #[test]
    fn plans_that_cannot_be_invoiced_are_refused() {
        // Cloud and Enterprise both reach here only by direct API call.
        assert!(plan_to_invoice(false, None, Some(get_enterprise_plan()), &purchasable()).is_err());
        assert!(plan_to_invoice(true, Some(get_free_plan()), None, &purchasable()).is_err());
    }
}
