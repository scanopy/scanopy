//! SSH detection by its identification string.
//!
//! RFC 4253 §4.2 requires the server to send `SSH-protocolversion-softwareversion` before anything
//! else happens, so this needs no request at all. That also makes it one of the cleanest separations
//! available between a real service and a listener that merely completes a handshake: a middlebox
//! answering on behalf of an empty address has nothing to send.

use anyhow::Error;
use async_trait::async_trait;

use crate::daemon::utils::app_probe::{
    AppProbe, AppProbeOutcome, DeviceIdentity, ProbeContext, read_greeting,
};
use crate::server::hosts::r#impl::attributes::HostOsValue;
use crate::server::hosts::r#impl::os::recog::RecogDatabase;
use crate::server::ports::r#impl::base::PortType;
use crate::server::services::r#impl::patterns::ClientProbe;
use crate::server::shared::attribution::{AttributeSource, Attributed};

/// The identification string every SSH server opens with.
const SSH_BANNER: &[u8] = b"SSH-";

pub struct SshProbe;

#[async_trait]
impl AppProbe for SshProbe {
    fn port(&self) -> PortType {
        PortType::Ssh
    }

    fn client_probe(&self) -> Option<ClientProbe> {
        Some(ClientProbe::Ssh)
    }

    async fn run(&self, ctx: &ProbeContext) -> Result<AppProbeOutcome, Error> {
        Ok(parse_banner(&read_greeting(ctx, self.port(), 512).await))
    }
}

/// Whether the opening bytes are an SSH identification string, and the OS it names, if any.
///
/// The version that follows is not checked: `SSH-1.99` and `SSH-2.0` are both real, and a server
/// free-texting its software name after them is expected. The prefix is what identifies the
/// protocol.
///
/// The software version after it is matched against Recog: a distribution's OpenSSH package
/// appends its own tag (`OpenSSH_9.6p1 Ubuntu-3ubuntu13.5`), and some servers name their OS
/// outright (`OpenSSH_for_Windows`, `ROSSSH`). A bare `OpenSSH_9.6` names nothing, and the
/// probe still answers.
fn parse_banner(bytes: &[u8]) -> AppProbeOutcome {
    if !bytes.starts_with(SSH_BANNER) {
        return AppProbeOutcome::NoAnswer;
    }
    let line = String::from_utf8_lossy(bytes);
    let line = line.lines().next().unwrap_or_default();
    let os = line
        .splitn(3, '-')
        .nth(2)
        .and_then(|software| RecogDatabase::SshBanner.os(software));
    let identity = os.map(|os| DeviceIdentity {
        manufacturer: None,
        model: None,
        serial_number: None,
        firmware_revision: None,
        os: Some(Attributed::new(
            HostOsValue(os),
            AttributeSource::SshBannerMatch,
        )),
    });
    AppProbeOutcome::Answered { identity }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_identification_string_is_ssh() {
        for banner in [
            &b"SSH-2.0-OpenSSH_9.6\r\n"[..],
            &b"SSH-1.99-Cisco-1.25\r\n"[..],
            &b"SSH-2.0-dropbear_2022.83\r\n"[..],
        ] {
            assert!(matches!(
                parse_banner(banner),
                AppProbeOutcome::Answered { .. }
            ));
        }
    }

    #[test]
    fn a_distribution_banner_names_the_os_and_a_bare_one_names_none() {
        let AppProbeOutcome::Answered { identity } =
            parse_banner(b"SSH-2.0-OpenSSH_6.6.1p1 Ubuntu-2ubuntu2.13\r\n")
        else {
            panic!("an identification string answers");
        };
        let os = identity
            .and_then(|i| i.os)
            .expect("the Ubuntu tag names an OS");
        assert_eq!(os.value().0.name.as_deref(), Some("Ubuntu"));
        assert_eq!(os.source(), AttributeSource::SshBannerMatch);

        assert_eq!(
            parse_banner(b"SSH-2.0-OpenSSH_9.6\r\n"),
            AppProbeOutcome::Answered { identity: None }
        );
    }

    #[test]
    fn silence_or_another_protocol_is_not_ssh() {
        for banner in [
            &b""[..],
            &b"220 ProFTPD Server ready\r\n"[..],
            &b"HTTP/1.1 400 Bad Request\r\n"[..],
            &b"\0\0\0\0"[..],
        ] {
            assert_eq!(parse_banner(banner), AppProbeOutcome::NoAnswer);
        }
    }
}
