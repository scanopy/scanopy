//! Amazon SES bounce and complaint feedback, delivered by Amazon SNS to
//! `POST /api/email/feedback/sns`.
//!
//! Every message is authenticated before anything acts on it: it must come
//! from the configured topic, carry a SignatureVersion 2 signature
//! (SHA256withRSA, PKCS#1 v1.5), and name a signing certificate and
//! subscription URL on an `sns.<region>.amazonaws.com` host. Checking happens
//! in [`verify`], which takes the public key as an argument so it runs without
//! the network; [`EmailFeedbackService`] fetches and caches the certificate.
//!
//! A complaint turns off every pausable email for the user it names. A bounce
//! changes nothing: SES's account-level suppression list already stops repeat
//! sends to an address that hard-bounced. Both are counted and logged without
//! the address.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::server::metrics::dependency::{Dependency, observe_http};
use anyhow::{Context, anyhow};
use base64ct::{Base64, Encoding};
use email_address::EmailAddress;
use rsa::RsaPublicKey;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use serde::Deserialize;
use sha2::Sha256;
use x509_cert::Certificate;
use x509_cert::der::{Decode, Encode};

use crate::server::shared::events::types::UpdateReason;
use crate::server::users::{r#impl::email_settings::EmailSettings, service::UserService};

/// Signing certificates kept in memory. SNS signs with one certificate per
/// region at a time, so a handful covers every rotation a process lives
/// through; the cap keeps a stream of distinct valid URLs from growing it.
const MAX_CACHED_SIGNING_KEYS: usize = 16;

/// The `Type` of an SNS message delivered to an HTTPS subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum SnsMessageType {
    Notification,
    SubscriptionConfirmation,
    UnsubscribeConfirmation,
}

impl SnsMessageType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Notification => "Notification",
            Self::SubscriptionConfirmation => "SubscriptionConfirmation",
            Self::UnsubscribeConfirmation => "UnsubscribeConfirmation",
        }
    }
}

/// The JSON body SNS POSTs to an HTTPS subscription.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SnsMessage {
    pub r#type: SnsMessageType,
    pub message_id: String,
    /// Present on subscription and unsubscribe confirmations.
    #[serde(default)]
    pub token: Option<String>,
    pub topic_arn: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub message: String,
    pub timestamp: String,
    pub signature_version: String,
    pub signature: String,
    #[serde(rename = "SigningCertURL")]
    pub signing_cert_url: String,
    #[serde(rename = "SubscribeURL", default)]
    pub subscribe_url: Option<String>,
    #[serde(rename = "UnsubscribeURL", default)]
    pub unsubscribe_url: Option<String>,
}

/// Why a message was refused. Never shown to the sender: the endpoint answers
/// a bare 400, and the reason goes to the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejection {
    Malformed,
    WrongTopic,
    UnsupportedSignatureVersion,
    UntrustedUrl,
    BadSignature,
}

impl Rejection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::WrongTopic => "wrong_topic",
            Self::UnsupportedSignatureVersion => "unsupported_signature_version",
            Self::UntrustedUrl => "untrusted_url",
            Self::BadSignature => "bad_signature",
        }
    }
}

/// What handling a message came to, for the handler to map onto a status.
#[derive(Debug)]
pub enum FeedbackError {
    /// The message is not an authentic SNS message for our topic. 400.
    Rejected(Rejection),
    /// The message may be authentic but could not be processed now, such as
    /// when the signing certificate could not be fetched. 5xx, so SNS retries.
    Unavailable(anyhow::Error),
}

impl From<Rejection> for FeedbackError {
    fn from(rejection: Rejection) -> Self {
        Self::Rejected(rejection)
    }
}

/// True for `https://sns.<region>.amazonaws.com/...`, where `<region>` is
/// lowercase letters, digits and hyphens, on the default port. Anything that
/// merely contains that host (a longer domain, userinfo, another scheme) fails.
fn is_sns_url(raw: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    let Some(region) = url
        .host_str()
        .and_then(|host| host.strip_prefix("sns."))
        .and_then(|rest| rest.strip_suffix(".amazonaws.com"))
    else {
        return false;
    };
    !region.is_empty()
        && region
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The checks that need no key: topic, signature version, and the hosts of
/// the URLs the server will fetch. Run before the certificate is fetched, so a
/// message for another topic never causes an outbound request.
pub fn check_envelope(msg: &SnsMessage, expected_topic_arn: &str) -> Result<(), Rejection> {
    if msg.topic_arn != expected_topic_arn {
        return Err(Rejection::WrongTopic);
    }
    if msg.signature_version != "2" {
        return Err(Rejection::UnsupportedSignatureVersion);
    }
    if !is_sns_url(&msg.signing_cert_url) {
        return Err(Rejection::UntrustedUrl);
    }
    if msg.r#type == SnsMessageType::SubscriptionConfirmation
        && !msg.subscribe_url.as_deref().is_some_and(is_sns_url)
    {
        return Err(Rejection::UntrustedUrl);
    }
    Ok(())
}

/// The string SNS signs: selected fields as `Key\nValue\n`, in byte order of
/// the key. Per
/// <https://docs.aws.amazon.com/sns/latest/dg/sns-verify-signature-of-message.html>.
pub fn string_to_sign(msg: &SnsMessage) -> Result<String, Rejection> {
    let mut fields: Vec<(&str, &str)> =
        vec![("Message", &msg.message), ("MessageId", &msg.message_id)];
    match msg.r#type {
        SnsMessageType::Notification => {
            if let Some(subject) = &msg.subject {
                fields.push(("Subject", subject));
            }
            fields.push(("Timestamp", &msg.timestamp));
        }
        SnsMessageType::SubscriptionConfirmation | SnsMessageType::UnsubscribeConfirmation => {
            let subscribe_url = msg.subscribe_url.as_deref().ok_or(Rejection::Malformed)?;
            let token = msg.token.as_deref().ok_or(Rejection::Malformed)?;
            fields.push(("SubscribeURL", subscribe_url));
            fields.push(("Timestamp", &msg.timestamp));
            fields.push(("Token", token));
        }
    }
    fields.push(("TopicArn", &msg.topic_arn));
    fields.push(("Type", msg.r#type.as_str()));

    Ok(fields
        .into_iter()
        .map(|(key, value)| format!("{key}\n{value}\n"))
        .collect())
}

/// Check `msg`'s SHA256withRSA signature against `key`.
pub fn verify_signature(msg: &SnsMessage, key: &RsaPublicKey) -> Result<(), Rejection> {
    let signed = string_to_sign(msg)?;
    let signature_bytes =
        Base64::decode_vec(msg.signature.trim()).map_err(|_| Rejection::Malformed)?;
    let signature =
        Signature::try_from(signature_bytes.as_slice()).map_err(|_| Rejection::Malformed)?;
    VerifyingKey::<Sha256>::new(key.clone())
        .verify(signed.as_bytes(), &signature)
        .map_err(|_| Rejection::BadSignature)
}

/// Every check on `msg`, given the public key of the certificate its
/// `SigningCertURL` names.
pub fn verify(
    msg: &SnsMessage,
    expected_topic_arn: &str,
    key: &RsaPublicKey,
) -> Result<(), Rejection> {
    check_envelope(msg, expected_topic_arn)?;
    verify_signature(msg, key)
}

/// The RSA public key of a PEM X.509 certificate. The SubjectPublicKeyInfo
/// crosses from `x509-cert` to `rsa` as DER, because the two crates depend on
/// different versions of `spki`.
pub fn public_key_from_certificate_pem(pem: &str) -> anyhow::Result<RsaPublicKey> {
    let der = pem::parse(pem).context("signing certificate is not PEM")?;
    let certificate =
        Certificate::from_der(der.contents()).context("signing certificate is not X.509")?;
    let spki = certificate
        .tbs_certificate()
        .subject_public_key_info()
        .to_der()
        .context("signing certificate has no encodable public key")?;
    RsaPublicKey::from_public_key_der(&spki).context("signing certificate key is not RSA")
}

/// SES's classification of a bounce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BounceType {
    Permanent,
    Transient,
    Undetermined,
}

impl BounceType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::Transient => "transient",
            Self::Undetermined => "undetermined",
        }
    }
}

/// The parts of an SES identity notification this server acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SesFeedback {
    Bounce {
        bounce_type: BounceType,
        recipients: Vec<String>,
    },
    Complaint {
        recipients: Vec<String>,
    },
    /// Deliveries, SES's subscription test message, and anything else.
    Ignored,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SesNotification {
    /// `notificationType` in identity feedback notifications, `eventType`
    /// when the topic is an event-publishing destination. The `bounce` and
    /// `complaint` objects are the same in both.
    #[serde(alias = "eventType")]
    notification_type: String,
    bounce: Option<SesBounce>,
    complaint: Option<SesComplaint>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SesBounce {
    bounce_type: String,
    #[serde(default)]
    bounced_recipients: Vec<SesRecipient>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SesComplaint {
    #[serde(default)]
    complained_recipients: Vec<SesRecipient>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SesRecipient {
    email_address: String,
}

/// Parse the `Message` of an SNS notification as an SES identity
/// notification. Per
/// <https://docs.aws.amazon.com/ses/latest/dg/notification-contents.html>.
pub fn parse_ses_notification(message: &str) -> anyhow::Result<SesFeedback> {
    let notification: SesNotification =
        serde_json::from_str(message).context("not an SES notification")?;
    let addresses = |recipients: Vec<SesRecipient>| {
        recipients
            .into_iter()
            .map(|r| r.email_address)
            .collect::<Vec<_>>()
    };
    Ok(match notification.notification_type.as_str() {
        "Bounce" => {
            let bounce = notification
                .bounce
                .ok_or_else(|| anyhow!("Bounce notification without a bounce object"))?;
            let bounce_type = match bounce.bounce_type.as_str() {
                "Permanent" => BounceType::Permanent,
                "Transient" => BounceType::Transient,
                _ => BounceType::Undetermined,
            };
            SesFeedback::Bounce {
                bounce_type,
                recipients: addresses(bounce.bounced_recipients),
            }
        }
        "Complaint" => {
            let complaint = notification
                .complaint
                .ok_or_else(|| anyhow!("Complaint notification without a complaint object"))?;
            SesFeedback::Complaint {
                recipients: addresses(complaint.complained_recipients),
            }
        }
        _ => SesFeedback::Ignored,
    })
}

/// Authenticates SNS deliveries for the configured topic and applies the SES
/// feedback they carry. Present only when `email_feedback_sns_topic_arn` is set.
pub struct EmailFeedbackService {
    topic_arn: String,
    user_service: Arc<UserService>,
    /// Fetches signing certificates and confirms subscriptions. Follows no
    /// redirects, so a fetch never leaves the SNS host that was checked.
    http: reqwest::Client,
    signing_keys: Mutex<HashMap<String, RsaPublicKey>>,
}

impl EmailFeedbackService {
    pub fn new(topic_arn: String, user_service: Arc<UserService>) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("could not build the SNS HTTP client")?;
        Ok(Self {
            topic_arn,
            user_service,
            http,
            signing_keys: Mutex::new(HashMap::new()),
        })
    }

    /// Authenticate one SNS delivery and act on it.
    pub async fn handle(&self, body: &str) -> Result<(), FeedbackError> {
        let msg: SnsMessage = serde_json::from_str(body).map_err(|_| Rejection::Malformed)?;
        check_envelope(&msg, &self.topic_arn)?;
        let key = self
            .signing_key(&msg.signing_cert_url)
            .await
            .map_err(FeedbackError::Unavailable)?;
        verify_signature(&msg, &key)?;

        match msg.r#type {
            SnsMessageType::SubscriptionConfirmation => {
                // check_envelope guarantees an SNS-hosted URL.
                let url = msg.subscribe_url.as_deref().unwrap_or_default();
                observe_http(Dependency::Sns, self.http.get(url).send())
                    .await
                    .and_then(|response| response.error_for_status())
                    .context("could not confirm the SNS subscription")
                    .map_err(FeedbackError::Unavailable)?;
                tracing::info!(topic_arn = %msg.topic_arn, "Confirmed the SES feedback SNS subscription");
            }
            SnsMessageType::UnsubscribeConfirmation => {
                record("unsubscribe_confirmation", "none");
                tracing::warn!(
                    topic_arn = %msg.topic_arn,
                    "The SES feedback SNS subscription was removed; bounces and complaints will stop arriving"
                );
            }
            SnsMessageType::Notification => match parse_ses_notification(&msg.message) {
                Ok(feedback) => self
                    .apply(feedback)
                    .await
                    .map_err(FeedbackError::Unavailable)?,
                // Authentic but not something this server reads. Retrying
                // would not change that, so it is acknowledged.
                // The top-level keys show which format arrived without
                // logging any values, which hold addresses.
                Err(e) => tracing::warn!(
                    message_id = %msg.message_id,
                    error = %e,
                    keys = %serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
                        &msg.message
                    )
                    .map(|fields| fields.keys().cloned().collect::<Vec<_>>().join(","))
                    .unwrap_or_default(),
                    "Ignoring an SNS notification that is not SES feedback"
                ),
            },
        }
        Ok(())
    }

    pub(crate) async fn apply(&self, feedback: SesFeedback) -> anyhow::Result<()> {
        match feedback {
            SesFeedback::Bounce {
                bounce_type,
                recipients,
            } => {
                for _ in &recipients {
                    record("bounce", bounce_type.as_str());
                }
                tracing::info!(
                    bounce_type = bounce_type.as_str(),
                    recipients = recipients.len(),
                    "SES reported a bounce"
                );
            }
            SesFeedback::Complaint { recipients } => {
                for address in &recipients {
                    record("complaint", "none");
                    let Ok(email) = EmailAddress::from_str(address) else {
                        continue;
                    };
                    let Some(user) = self.user_service.get_by_email(&email).await? else {
                        tracing::info!("SES reported a complaint from an address with no account");
                        continue;
                    };
                    self.user_service
                        .update_email_settings(
                            user,
                            UpdateReason::UserEmailComplaintReceived,
                            EmailSettings::pause_all,
                        )
                        .await?;
                }
            }
            SesFeedback::Ignored => {}
        }
        Ok(())
    }

    /// The public key of the certificate at `url`, from the cache or fetched.
    /// Only called for a URL that passed [`check_envelope`].
    async fn signing_key(&self, url: &str) -> anyhow::Result<RsaPublicKey> {
        if let Some(key) = self.cached_key(url) {
            return Ok(key);
        }
        let pem = observe_http(Dependency::Sns, self.http.get(url).send())
            .await
            .and_then(|response| response.error_for_status())
            .context("could not fetch the SNS signing certificate")?
            .text()
            .await
            .context("could not read the SNS signing certificate")?;
        let key = public_key_from_certificate_pem(&pem)?;

        let mut keys = self
            .signing_keys
            .lock()
            .map_err(|_| anyhow!("signing key cache poisoned"))?;
        if keys.len() >= MAX_CACHED_SIGNING_KEYS {
            keys.clear();
        }
        keys.insert(url.to_string(), key.clone());
        Ok(key)
    }

    fn cached_key(&self, url: &str) -> Option<RsaPublicKey> {
        self.signing_keys.lock().ok()?.get(url).cloned()
    }
}

/// Bounded labels: `type` is bounce, complaint, or `unsubscribe_confirmation` (the
/// subscription was removed and feedback will stop arriving); `bounce_type` is one of the
/// three SES bounce types, or `none` for the other two.
fn record(feedback_type: &'static str, bounce_type: &'static str) {
    metrics::counter!(
        "scanopy_email_feedback_total",
        "type" => feedback_type,
        "bounce_type" => bounce_type,
    )
    .increment(1);
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use rsa::RsaPrivateKey;
    use rsa::pkcs1v15::SigningKey;
    use rsa::rand_core::OsRng;
    use rsa::signature::{SignatureEncoding, Signer};

    use super::*;

    const TOPIC: &str = "arn:aws:sns:us-east-1:123456789012:ses-feedback";
    const CERT_URL: &str =
        "https://sns.us-east-1.amazonaws.com/SimpleNotificationService-0000000000000000000000.pem";

    /// One key for the whole module: 2048-bit generation is slow in a debug build.
    fn private_key() -> &'static RsaPrivateKey {
        static KEY: OnceLock<RsaPrivateKey> = OnceLock::new();
        KEY.get_or_init(|| RsaPrivateKey::new(&mut OsRng, 2048).unwrap())
    }

    fn public_key() -> RsaPublicKey {
        private_key().to_public_key()
    }

    fn sign(msg: &mut SnsMessage) {
        let signer = SigningKey::<Sha256>::new(private_key().clone());
        let signed = string_to_sign(msg).unwrap();
        msg.signature = Base64::encode_string(&signer.sign(signed.as_bytes()).to_vec());
    }

    fn notification(message: &str) -> SnsMessage {
        let mut msg = SnsMessage {
            r#type: SnsMessageType::Notification,
            message_id: "22b80b92-fdea-4c2c-8f9d-bdfb0c7bf324".to_string(),
            token: None,
            topic_arn: TOPIC.to_string(),
            subject: Some("Amazon SES Email Event Notification".to_string()),
            message: message.to_string(),
            timestamp: "2012-05-02T00:54:06.655Z".to_string(),
            signature_version: "2".to_string(),
            signature: String::new(),
            signing_cert_url: CERT_URL.to_string(),
            subscribe_url: None,
            unsubscribe_url: Some(
                "https://sns.us-east-1.amazonaws.com/?Action=Unsubscribe".to_string(),
            ),
        };
        sign(&mut msg);
        msg
    }

    fn subscription_confirmation() -> SnsMessage {
        let mut msg = SnsMessage {
            r#type: SnsMessageType::SubscriptionConfirmation,
            message_id: "165545c9-2a5c-472c-8df2-7ff2be2b3b1b".to_string(),
            token: Some("2336412f37fb687f5d51e6e241d09c805a5a57b30d712f794cc5f6a988666d92768dd60a747ba6f3beb71854e285d6ad02428b09ceece29417f1f02d609c582afbacc99c583a916b9981dd2728f4ae6fdb82efd087cc3b7849e05798d2d2785c03b0879594eeac82c01f235d0e717736".to_string()),
            topic_arn: TOPIC.to_string(),
            subject: None,
            message: format!("You have chosen to subscribe to the topic {TOPIC}."),
            timestamp: "2012-04-26T20:45:04.751Z".to_string(),
            signature_version: "2".to_string(),
            signature: String::new(),
            signing_cert_url: CERT_URL.to_string(),
            subscribe_url: Some(
                "https://sns.us-east-1.amazonaws.com/?Action=ConfirmSubscription&TopicArn=x&Token=y"
                    .to_string(),
            ),
            unsubscribe_url: None,
        };
        sign(&mut msg);
        msg
    }

    #[test]
    fn a_correctly_signed_message_verifies() {
        assert_eq!(verify(&notification("{}"), TOPIC, &public_key()), Ok(()));
        assert_eq!(
            verify(&subscription_confirmation(), TOPIC, &public_key()),
            Ok(())
        );
    }

    /// A notification without a Subject signs without the Subject pair.
    #[test]
    fn a_notification_without_a_subject_verifies() {
        let mut msg = notification("{}");
        msg.subject = None;
        sign(&mut msg);
        assert_eq!(verify(&msg, TOPIC, &public_key()), Ok(()));
    }

    #[test]
    fn any_signed_field_changed_after_signing_fails() {
        let tamperings: Vec<fn(&mut SnsMessage)> = vec![
            |m| m.message = r#"{"notificationType":"Complaint"}"#.to_string(),
            |m| m.message_id.push('0'),
            |m| m.subject = Some("Other".to_string()),
            |m| m.subject = None,
            |m| m.timestamp = "2012-05-02T00:54:07.655Z".to_string(),
            |m| m.r#type = SnsMessageType::UnsubscribeConfirmation,
        ];
        for tamper in tamperings {
            let mut msg = notification("{}");
            tamper(&mut msg);
            assert!(verify(&msg, TOPIC, &public_key()).is_err(), "{msg:?}");
        }

        let mut msg = subscription_confirmation();
        msg.subscribe_url = Some("https://sns.us-east-1.amazonaws.com/?Action=Other".to_string());
        assert_eq!(
            verify(&msg, TOPIC, &public_key()),
            Err(Rejection::BadSignature)
        );
    }

    #[test]
    fn a_signature_from_another_key_fails() {
        let other = RsaPrivateKey::new(&mut OsRng, 1024)
            .unwrap()
            .to_public_key();
        assert_eq!(
            verify(&notification("{}"), TOPIC, &other),
            Err(Rejection::BadSignature)
        );
    }

    #[test]
    fn another_topic_is_refused_even_when_correctly_signed() {
        let mut msg = notification("{}");
        msg.topic_arn = "arn:aws:sns:us-east-1:999999999999:someone-else".to_string();
        sign(&mut msg);
        assert_eq!(
            verify(&msg, TOPIC, &public_key()),
            Err(Rejection::WrongTopic)
        );
    }

    #[test]
    fn signature_version_1_is_refused() {
        let mut msg = notification("{}");
        msg.signature_version = "1".to_string();
        assert_eq!(
            verify(&msg, TOPIC, &public_key()),
            Err(Rejection::UnsupportedSignatureVersion)
        );
    }

    #[test]
    fn certificate_and_subscribe_urls_must_be_sns_hosts() {
        for url in [
            "https://evil.com/cert.pem",
            "https://sns.us-east-1.amazonaws.com.evil.com/cert.pem",
            "http://sns.us-east-1.amazonaws.com/cert.pem",
            "https://sns.us-east-1.amazonaws.com:8443/cert.pem",
            "https://sns.us-east-1.amazonaws.com@evil.com/cert.pem",
            "https://sns..amazonaws.com/cert.pem",
            "https://sns.US-EAST-1.amazonaws.com.evil.com/cert.pem",
            "https://evilsns.us-east-1.amazonaws.com/cert.pem",
            "not a url",
        ] {
            let mut msg = notification("{}");
            msg.signing_cert_url = url.to_string();
            assert_eq!(
                verify(&msg, TOPIC, &public_key()),
                Err(Rejection::UntrustedUrl),
                "{url}"
            );

            let mut msg = subscription_confirmation();
            msg.subscribe_url = Some(url.to_string());
            sign(&mut msg);
            assert_eq!(
                verify(&msg, TOPIC, &public_key()),
                Err(Rejection::UntrustedUrl),
                "{url}"
            );
        }

        assert!(is_sns_url("https://sns.ap-southeast-2.amazonaws.com/x.pem"));
    }

    #[test]
    fn a_subscription_confirmation_without_its_token_is_malformed() {
        let mut msg = subscription_confirmation();
        msg.token = None;
        assert_eq!(
            verify(&msg, TOPIC, &public_key()),
            Err(Rejection::Malformed)
        );
    }

    /// The body SNS POSTs deserializes, including the URL fields whose names
    /// are not plain PascalCase.
    #[test]
    fn an_sns_body_parses() {
        let body = format!(
            r#"{{
                "Type" : "Notification",
                "MessageId" : "22b80b92-fdea-4c2c-8f9d-bdfb0c7bf324",
                "TopicArn" : "{TOPIC}",
                "Subject" : "My First Message",
                "Message" : "Hello world!",
                "Timestamp" : "2012-05-02T00:54:06.655Z",
                "SignatureVersion" : "2",
                "Signature" : "EXAMPLE",
                "SigningCertURL" : "{CERT_URL}",
                "UnsubscribeURL" : "https://sns.us-east-1.amazonaws.com/?Action=Unsubscribe"
            }}"#
        );
        let msg: SnsMessage = serde_json::from_str(&body).unwrap();
        assert_eq!(msg.r#type, SnsMessageType::Notification);
        assert_eq!(msg.signing_cert_url, CERT_URL);
        assert_eq!(check_envelope(&msg, TOPIC), Ok(()));
    }

    /// From the SES notification contents reference.
    const SES_BOUNCE: &str = r#"{
        "notificationType":"Bounce",
        "bounce":{
            "bounceType":"Permanent",
            "bounceSubType":"General",
            "bouncedRecipients":[
                {"status":"5.0.0","action":"failed","diagnosticCode":"smtp; 550 user unknown","emailAddress":"recipient1@example.com"},
                {"status":"4.0.0","action":"delayed","emailAddress":"recipient2@example.com"}
            ],
            "reportingMTA":"example.com",
            "timestamp":"2012-05-25T14:59:38.605Z",
            "feedbackId":"000001378603176d-5a4b5ad9-6f30-4198-a8c3-b1eb0c270a1d-000000",
            "remoteMtaIp":"127.0.2.0"
        },
        "mail":{
            "timestamp":"2018-10-08T14:05:45 +0000",
            "messageId":"000001378603177f-7a5433e7-8edb-42ae-af10-f0181f34d6ee-000000",
            "source":"sender@example.com",
            "sourceArn":"arn:aws:ses:us-east-1:888888888888:identity/example.com",
            "sourceIp":"127.0.3.0",
            "sendingAccountId":"123456789012",
            "destination":["recipient1@example.com","recipient2@example.com"]
        }
    }"#;

    const SES_COMPLAINT: &str = r#"{
        "notificationType":"Complaint",
        "complaint":{
            "userAgent":"ExampleCorp Feedback Loop (V0.01)",
            "complainedRecipients":[{"emailAddress":"recipient1@example.com"}],
            "complaintFeedbackType":"abuse",
            "arrivalDate":"2009-12-03T04:24:21.000-05:00",
            "timestamp":"2012-05-25T14:59:38.623Z",
            "feedbackId":"000001378603177f-18c07c78-fa81-4a58-9dd1-fedc3cb8f49a-000000"
        },
        "mail":{
            "timestamp":"2018-10-08T14:05:45 +0000",
            "messageId":"000001378603177f-7a5433e7-8edb-42ae-af10-f0181f34d6ee-000000",
            "source":"sender@example.com",
            "destination":["recipient1@example.com"]
        }
    }"#;

    #[test]
    fn ses_bounces_and_complaints_parse() {
        assert_eq!(
            parse_ses_notification(SES_BOUNCE).unwrap(),
            SesFeedback::Bounce {
                bounce_type: BounceType::Permanent,
                recipients: vec![
                    "recipient1@example.com".to_string(),
                    "recipient2@example.com".to_string()
                ],
            }
        );
        assert_eq!(
            parse_ses_notification(SES_COMPLAINT).unwrap(),
            SesFeedback::Complaint {
                recipients: vec!["recipient1@example.com".to_string()],
            }
        );
    }

    /// An event-publishing destination names the type `eventType` and is
    /// otherwise the same, so either setup yields the same feedback.
    #[test]
    fn event_publishing_records_parse_like_identity_notifications() {
        for sample in [SES_BOUNCE, SES_COMPLAINT] {
            let event = sample.replace("\"notificationType\"", "\"eventType\"");
            assert_eq!(
                parse_ses_notification(&event).unwrap(),
                parse_ses_notification(sample).unwrap()
            );
        }
    }

    #[test]
    fn deliveries_and_the_ses_setup_message_are_ignored() {
        let delivery = r#"{"notificationType":"Delivery","mail":{},"delivery":{"recipients":["a@example.com"]}}"#;
        let setup = r#"{"notificationType":"AmazonSnsSubscriptionSucceeded","message":"You have successfully subscribed"}"#;
        assert_eq!(
            parse_ses_notification(delivery).unwrap(),
            SesFeedback::Ignored
        );
        assert_eq!(parse_ses_notification(setup).unwrap(), SesFeedback::Ignored);
        assert!(parse_ses_notification("Hello world!").is_err());
    }

    #[test]
    fn an_unrecognised_bounce_type_counts_as_undetermined() {
        let message = SES_BOUNCE.replace("\"Permanent\"", "\"SomethingNew\"");
        assert!(matches!(
            parse_ses_notification(&message).unwrap(),
            SesFeedback::Bounce {
                bounce_type: BounceType::Undetermined,
                ..
            }
        ));
    }
}
