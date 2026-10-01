//! The Stripe client every billing call goes through, held under Stripe's
//! rate limits.
//!
//! Stripe limits requests per account, across every caller: 100 a second
//! live, 25 in a sandbox, and 25 a second on any one endpoint (20 on Files and
//! Search). Over a limit it answers 429. One token bucket per process covers
//! all of them: its rate sits at or below the lowest per-endpoint limit, so no
//! endpoint can be pushed past its own limit even when all of the traffic goes
//! to it. Per-endpoint buckets would need the request's path, which
//! `CustomizedStripeRequest` gives up only by being taken apart.
use bytes::Bytes;
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use stripe::{Client, StripeError};
use stripe_client_core::{CustomizedStripeRequest, StripeClient};

/// Requests a second one process sends to a live account. The Files limit,
/// 20, is the lowest per-endpoint limit we reach, and two containers during a
/// rolling deploy stay well under the global 100.
const LIVE_REQUESTS_PER_SECOND: u32 = 20;
/// Requests a second one process sends to a sandbox, whose global limit is 25.
const SANDBOX_REQUESTS_PER_SECOND: u32 = 10;
/// How long a request waits for a permit before giving up. A webhook that
/// gives up answers with an error and Stripe redelivers it.
const PERMIT_WAIT: Duration = Duration::from_secs(10);

/// Which of Stripe's limits the account's key falls under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripeMode {
    Live,
    Sandbox,
}

impl StripeMode {
    /// Read from the key's prefix: `sk_live_`/`rk_live_` are live, and
    /// anything else gets the sandbox's lower rate.
    pub fn from_secret(secret: &str) -> Self {
        if secret.starts_with("sk_live_") || secret.starts_with("rk_live_") {
            Self::Live
        } else {
            Self::Sandbox
        }
    }

    fn requests_per_second(self) -> u32 {
        match self {
            Self::Live => LIVE_REQUESTS_PER_SECOND,
            Self::Sandbox => SANDBOX_REQUESTS_PER_SECOND,
        }
    }

    fn quota(self) -> Quota {
        Quota::per_second(NonZeroU32::new(self.requests_per_second()).expect("rate is non-zero"))
    }
}

#[derive(Clone)]
pub struct RateLimitedStripe {
    inner: Client,
    limiter: Arc<DefaultDirectRateLimiter>,
    permit_wait: Duration,
}

impl RateLimitedStripe {
    pub fn new(secret: &str) -> Self {
        let mode = StripeMode::from_secret(secret);
        tracing::info!(
            stripe_mode = ?mode,
            requests_per_second = mode.requests_per_second(),
            "Stripe client rate limit"
        );
        Self::with_quota(Client::new(secret), mode.quota(), PERMIT_WAIT)
    }

    fn with_quota(inner: Client, quota: Quota, permit_wait: Duration) -> Self {
        Self {
            inner,
            limiter: Arc::new(RateLimiter::direct(quota)),
            permit_wait,
        }
    }

    /// Wait for a permit. Requests to Stripe outside this client, such as file
    /// downloads, take one here so that they count against the same limit.
    pub async fn acquire(&self) -> Result<(), StripeError> {
        tokio::time::timeout(self.permit_wait, self.limiter.until_ready())
            .await
            .map_err(|_| {
                tracing::warn!(
                    waited = ?self.permit_wait,
                    "Stripe request rate limit reached; not sending the request"
                );
                StripeError::Timeout
            })
    }
}

impl StripeClient for RateLimitedStripe {
    type Err = StripeError;

    async fn execute(&self, req: CustomizedStripeRequest) -> Result<Bytes, StripeError> {
        self.acquire().await?;
        self.inner.execute(req).await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use stripe_billing::invoice::RetrieveInvoice;
    use stripe_billing::subscription::RetrieveSubscription;

    /// A local stand-in for the Stripe API, serving `router`.
    pub(crate) async fn fake_stripe(router: Router) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (url, server)
    }

    pub(crate) fn client_for(url: &str, quota: Quota, permit_wait: Duration) -> RateLimitedStripe {
        let inner = stripe::ClientBuilder::new("sk_test_fake")
            .url(url)
            .build()
            .unwrap();
        RateLimitedStripe::with_quota(inner, quota, permit_wait)
    }

    #[test]
    fn only_a_live_key_gets_the_live_rate() {
        assert_eq!(StripeMode::from_secret("sk_live_abc"), StripeMode::Live);
        assert_eq!(StripeMode::from_secret("rk_live_abc"), StripeMode::Live);
        assert_eq!(StripeMode::from_secret("sk_test_abc"), StripeMode::Sandbox);
        assert_eq!(StripeMode::from_secret("rk_test_abc"), StripeMode::Sandbox);
        // A malformed or unfamiliar key is held to the lower rate.
        assert_eq!(StripeMode::from_secret("whsec_abc"), StripeMode::Sandbox);
        assert_eq!(StripeMode::from_secret(""), StripeMode::Sandbox);
    }

    /// Stripe's lowest per-endpoint limit is 20 a second and a sandbox's
    /// global limit is 25. A process at or under these cannot hit either on
    /// its own.
    #[test]
    fn no_mode_exceeds_a_single_endpoint_limit() {
        for mode in [StripeMode::Live, StripeMode::Sandbox] {
            assert!(mode.requests_per_second() <= 20, "{mode:?}");
        }
        assert!(StripeMode::Sandbox.requests_per_second() < 25);
    }

    /// The webhook path fetches each event's object in our own API version.
    /// These are a subscription and an invoice as Stripe returned them in
    /// that version, recorded for `evt_1ULZRvAkcpHEg5IAyzQrl0Oa`.
    #[tokio::test]
    async fn objects_fetched_in_our_version_parse() {
        let router = Router::new()
            .route(
                "/v1/subscriptions/{id}",
                get(|| async { include_str!("fixtures/subscription.dahlia.json") }),
            )
            .route(
                "/v1/invoices/{id}",
                get(|| async { include_str!("fixtures/invoice.dahlia.json") }),
            );
        let (url, server) = fake_stripe(router).await;
        let quota = Quota::per_second(NonZeroU32::new(100).unwrap());
        let client = client_for(&url, quota, Duration::from_secs(1));

        let subscription = RetrieveSubscription::new("sub_1ULZRtAkcpHEg5IAGeVo9uIn")
            .send(&client)
            .await
            .unwrap();
        assert!(subscription.metadata.contains_key("plan"));
        let invoice = RetrieveInvoice::new("in_1ULZRtAkcpHEg5IATS8EybqB")
            .send(&client)
            .await
            .unwrap();
        assert!(invoice.customer.is_some());
        server.abort();
    }

    #[tokio::test]
    async fn a_request_over_the_rate_is_not_sent() {
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        let router = Router::new().route(
            "/{*path}",
            get(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                async { "{}" }
            }),
        );
        let (url, server) = fake_stripe(router).await;
        let quota = Quota::per_hour(NonZeroU32::new(1).unwrap());
        let client = client_for(&url, quota, Duration::from_millis(100));

        // The first request spends the only permit. Its body doesn't parse as
        // a subscription, which doesn't matter here: it reached the server.
        let _ = RetrieveSubscription::new("sub_1").send(&client).await;
        let second = RetrieveSubscription::new("sub_2").send(&client).await;

        assert!(matches!(second, Err(StripeError::Timeout)));
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        server.abort();
    }
}
