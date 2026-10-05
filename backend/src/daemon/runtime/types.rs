use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::daemon::{
    shared::{config::ConfigStore, services::DaemonServiceFactory},
    utils::base::PlatformDaemonUtils,
};

#[derive(Serialize, Deserialize)]
pub struct InitializeDaemonRequest {
    pub site_id: Uuid,
    pub api_key: String,
}

pub struct DaemonAppState {
    pub config: Arc<ConfigStore>,
    pub services: Arc<DaemonServiceFactory>,
    pub utils: PlatformDaemonUtils,
}

impl DaemonAppState {
    pub async fn new(
        config: Arc<ConfigStore>,
        utils: PlatformDaemonUtils,
    ) -> anyhow::Result<Arc<Self>> {
        config.initialize().await?;

        let services = Arc::new(DaemonServiceFactory::new(config.clone()).await?);
        Ok(Arc::new(Self {
            config,
            services,
            utils,
        }))
    }
}
