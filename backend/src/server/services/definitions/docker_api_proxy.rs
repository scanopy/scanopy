use crate::server::services::definitions::{ServiceDefinitionFactory, create_service};
use crate::server::services::r#impl::categories::ServiceCategory;
use crate::server::services::r#impl::definitions::ServiceDefinition;
use crate::server::services::r#impl::patterns::Pattern;

/// A container serving the Docker API on a TCP port in front of the runtime's socket (a socket
/// proxy such as `tecnativa/docker-socket-proxy`, or socat).
///
/// Never matched from the network: the API it answers with is the runtime's own, so nothing in a
/// response tells the proxy from the daemon. The container integration attaches it to the
/// container that publishes the port the Docker proxy credential connects to.
#[derive(Default, Clone, Eq, PartialEq, Hash)]
pub struct DockerApiProxy;

impl ServiceDefinition for DockerApiProxy {
    fn name(&self) -> &'static str {
        "Docker API Proxy"
    }
    fn description(&self) -> &'static str {
        "A container that serves the Docker API on a TCP port in front of the runtime's socket"
    }
    fn category(&self) -> ServiceCategory {
        ServiceCategory::Container
    }

    fn discovery_pattern(&self) -> Pattern<'_> {
        Pattern::None
    }

    fn logo_url(&self) -> &'static str {
        "https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/docker.svg"
    }
}

inventory::submit!(ServiceDefinitionFactory::new(
    create_service::<DockerApiProxy>
));
