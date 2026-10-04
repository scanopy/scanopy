use crate::server::ports::r#impl::base::PortType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::{fmt::Display, net::IpAddr};
use strum::IntoDiscriminant;
use strum_macros::{Display, EnumDiscriminants, EnumIter};

#[derive(
    Debug,
    Copy,
    Clone,
    Default,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    EnumDiscriminants,
    EnumIter,
)]
#[strum_discriminants(derive(Display, Hash, Serialize, Deserialize, EnumIter, PartialOrd, Ord))]
pub enum ApplicationProtocol {
    #[default]
    Http,
    Https,
}

impl Display for ApplicationProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ApplicationProtocol::Http => "http",
            ApplicationProtocol::Https => "https",
        };

        write!(f, "{}", str)
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Endpoint {
    pub protocol: ApplicationProtocol,
    pub ip: Option<IpAddr>,
    pub port_type: PortType,
    pub path: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EndpointResponse {
    pub endpoint: Endpoint,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub status: u16,
}

impl EndpointResponse {
    /// Whether the body contains `needle` (case-insensitive) anywhere other than in an echo of the
    /// requested path.
    ///
    /// Echo servers, debug pages and some reverse proxies write the request line back into the
    /// body, so a pattern that requests `/zabbix` and looks for "zabbix" matched every one of them.
    /// The path is removed before searching: an echo of the request says nothing about what is
    /// serving it, while a real Zabbix page still names itself elsewhere.
    pub fn body_contains_beyond_request(&self, needle: &str) -> bool {
        let body = self.body.to_lowercase();
        let needle = needle.to_lowercase();
        let path = self.endpoint.path.to_lowercase();
        if path.is_empty() || path == "/" {
            return body.contains(&needle);
        }
        body.replace(&path, " ").contains(&needle)
    }
}

impl Display for EndpointResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let body_length = self.body.len().min(20);
        write!(
            f,
            "Response from {} -- Status {}; Headers {:?}; Body {}...",
            self.endpoint,
            self.status,
            self.headers,
            &self.body[0..body_length]
        )
    }
}

impl Endpoint {
    pub fn is_resolved(&self) -> bool {
        self.ip.is_some()
    }

    pub fn use_ip(&self, ip: IpAddr) -> Self {
        Self {
            protocol: self.protocol,
            ip: Some(ip),
            port_type: self.port_type,
            path: self.path.clone(),
        }
    }

    pub fn http(ip: Option<IpAddr>, path: &str) -> Self {
        Endpoint {
            protocol: ApplicationProtocol::Http,
            port_type: PortType::Http,
            ip,
            path: path.to_string(),
        }
    }

    pub fn for_pattern(port_type: PortType, path: &str) -> Self {
        Endpoint {
            protocol: ApplicationProtocol::Http,
            ip: None,
            port_type,
            path: path.to_owned(),
        }
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.ip {
            Some(ip) => {
                write!(
                    f,
                    "{}://{}:{}{}",
                    self.protocol.discriminant().to_string().to_lowercase(),
                    ip,
                    self.port_type.number(),
                    self.path
                )
            }
            None => {
                write!(
                    f,
                    "{}://<unresolved>:{}{}",
                    self.protocol.discriminant().to_string().to_lowercase(),
                    self.port_type.number(),
                    self.path
                )
            }
        }
    }
}

impl PartialEq for Endpoint {
    fn eq(&self, other: &Self) -> bool {
        self.protocol == other.protocol
            && self.port_type.number() == other.port_type.number()
            && self.path == other.path
    }
}

impl Hash for Endpoint {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.protocol.hash(state);
        self.port_type.hash(state);
        self.path.hash(state);
    }
}
