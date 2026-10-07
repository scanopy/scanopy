//! Dependency-group membership updates.
use super::*;

impl ServiceService {
    pub(crate) async fn update_dependency_members(
        &self,
        current_service: &Service,
        updates: Option<&Service>,
        authenticated: AuthenticatedEntity,
    ) -> Result<(), Error> {
        use crate::server::dependencies::r#impl::base::DependencyMembers;

        // DB-level lock, per site (replaces the old process-global mutex).
        // Acquired BEFORE the read: the dependency list feeds the
        // read-modify-write below, so reading outside the lock would let two
        // callers serialize conflicting writes computed from the same stale
        // snapshot. Error paths release via Drop.
        let lock_guard = self
            .storage
            .session_lock(
                LockKey::DependencyMembers {
                    site_id: current_service.base.site_id,
                },
                DEFAULT_LOCK_TIMEOUT,
            )
            .await?;

        let filter =
            StorableFilter::<Dependency>::new_from_site_ids(&[current_service.base.site_id]);
        let dependencies = self.dependency_service.get_all(filter).await?;

        let current_service_binding_ids: Vec<Uuid> = current_service
            .base
            .bindings
            .iter()
            .map(|b| b.id())
            .collect();
        let updated_service_binding_ids: Vec<Uuid> = match updates {
            Some(updated_service) => updated_service
                .base
                .bindings
                .iter()
                .map(|b| b.id())
                .collect(),
            None => Vec::new(),
        };

        let is_service_deleted = updates.is_none();

        let deps_to_update: Vec<Dependency> = dependencies
            .into_iter()
            .filter_map(|mut dep| {
                let changed = match &mut dep.base.members {
                    DependencyMembers::Services { service_ids } => {
                        if is_service_deleted {
                            let initial_len = service_ids.len();
                            service_ids.retain(|id| *id != current_service.id);
                            service_ids.len() != initial_len
                        } else {
                            false // Service updates don't affect service-level deps
                        }
                    }
                    DependencyMembers::Bindings { binding_ids } => {
                        let initial_len = binding_ids.len();
                        if is_service_deleted {
                            // Remove all bindings that belonged to this service
                            binding_ids.retain(|bid| !current_service_binding_ids.contains(bid));
                        } else {
                            // Remove bindings that were in the old service but not the new
                            binding_ids.retain(|bid| {
                                let in_current = current_service_binding_ids.contains(bid);
                                let in_updated = updated_service_binding_ids.contains(bid);
                                if in_current { in_updated } else { true }
                            });
                        }
                        binding_ids.len() != initial_len
                    }
                };

                if changed { Some(dep) } else { None }
            })
            .collect();

        if !deps_to_update.is_empty() {
            for mut dep in deps_to_update {
                self.dependency_service
                    .update(&mut dep, authenticated.clone())
                    .await?;
            }
        }

        lock_guard.release().await?;
        Ok(())
    }
}
