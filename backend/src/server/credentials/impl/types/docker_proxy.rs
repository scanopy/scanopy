//! Docker proxy credential types for discovery dispatch.

use crate::server::credentials::r#impl::mapping::{
    BannerField, BannerFieldValue, ResolvableSecret, ResolvableValue,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct DockerProxyQueryCredential {
    pub port: u16,
    pub path: Option<String>,
    pub ssl_cert: Option<ResolvableValue>,
    pub ssl_key: Option<ResolvableSecret>,
    pub ssl_chain: Option<ResolvableValue>,
}

/// Banner lines for Docker proxy credentials
impl DockerProxyQueryCredential {
    pub fn banner_lines(&self) -> Vec<BannerField> {
        let mut lines = vec![BannerField {
            label: "Port",
            value: BannerFieldValue::Plain(self.port.to_string()),
        }];
        if let Some(ref path) = self.path {
            lines.push(BannerField {
                label: "Path",
                value: BannerFieldValue::Plain(path.clone()),
            });
        }
        if let Some(ref cert) = self.ssl_cert {
            lines.push(BannerField {
                label: "SSL cert",
                value: cert.banner_value(),
            });
        }
        if let Some(ref key) = self.ssl_key {
            lines.push(BannerField {
                label: "SSL key",
                value: key.banner_value(),
            });
        }
        if let Some(ref chain) = self.ssl_chain {
            lines.push(BannerField {
                label: "SSL chain",
                value: chain.banner_value(),
            });
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::credentials::r#impl::mapping::CredentialQueryPayload;

    #[test]
    fn banner_lines_docker_proxy() {
        let payload = CredentialQueryPayload::DockerProxy(DockerProxyQueryCredential {
            port: 2376,
            path: Some("/".to_string()),
            ssl_cert: Some(ResolvableValue::Value {
                value: "cert-content".to_string(),
            }),
            ssl_key: Some(ResolvableSecret::FilePath {
                path: "/nonexistent/key.pem".to_string(),
            }),
            ssl_chain: None,
        });
        let lines = payload.banner_lines();
        assert_eq!(lines.len(), 4); // port, path, ssl_cert, ssl_key
        assert_eq!(lines[0].label, "Port");
        assert!(matches!(&lines[0].value, BannerFieldValue::Plain(v) if v == "2376"));
        assert_eq!(lines[1].label, "Path");
        assert_eq!(lines[2].label, "SSL cert");
        // Short inline values show as Plain
        assert!(matches!(&lines[2].value, BannerFieldValue::Plain(v) if v == "cert-content"));
        assert_eq!(lines[3].label, "SSL key");
        assert!(lines[3].value.is_failed());
    }
}
