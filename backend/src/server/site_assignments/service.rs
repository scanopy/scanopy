use crate::server::shared::relations as R;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::server::{
    auth::access::{Assignment, AssignmentScope},
    shared::storage::{
        filter::StorableFilter,
        generic::GenericPostgresStorage,
        traits::{Storable, Storage},
    },
    site_assignments::r#impl::base::{Principal, SiteAssignment, SiteAssignmentBase},
};

/// The `site_assignments` table. Writes come from `AccessService`, which owns the rules around them.
pub struct SiteAssignmentService {
    storage: Arc<GenericPostgresStorage<SiteAssignment>>,
}

impl SiteAssignmentService {
    pub fn new(storage: Arc<GenericPostgresStorage<SiteAssignment>>) -> Self {
        Self { storage }
    }

    pub async fn get_for(&self, principal: Principal) -> Result<Vec<Assignment>> {
        let rows = self.storage.get_for(&principal.assignments()).await?;
        Ok(rows.into_iter().map(|row| row.base.assignment).collect())
    }

    /// The sites listed for `principal`. An All sites row names none.
    pub async fn listed_site_ids(&self, principal: Principal) -> Result<Vec<Uuid>> {
        let rows = self.storage.get_for(&principal.assignments()).await?;
        Ok(rows.iter().filter_map(SiteAssignment::site_id).collect())
    }

    /// [`Self::listed_site_ids`] for several principals of one type, by the id in `column`.
    pub async fn listed_site_ids_by(
        &self,
        column: &str,
        ids: &[Uuid],
        id_of: impl Fn(&Principal) -> Option<Uuid>,
    ) -> Result<HashMap<Uuid, Vec<Uuid>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<SiteAssignment>::new().uuids_column(column, ids);
        let mut listed: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for row in self.storage.get_all(filter).await? {
            if let (Some(id), Some(site_id)) = (id_of(&row.base.principal), row.site_id()) {
                listed.entry(id).or_default().push(site_id);
            }
        }
        Ok(listed)
    }

    /// What an API key's rows come to as its `site_ids`, for several principals of one type by the
    /// id in `column`: `None` for one holding an All sites row, otherwise its listed sites. A
    /// principal with no rows is missing from the map.
    pub async fn site_scopes_by(
        &self,
        column: &str,
        ids: &[Uuid],
        id_of: impl Fn(&Principal) -> Option<Uuid>,
    ) -> Result<HashMap<Uuid, Option<Vec<Uuid>>>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let filter = StorableFilter::<SiteAssignment>::new().uuids_column(column, ids);
        let mut scopes: HashMap<Uuid, Option<Vec<Uuid>>> = HashMap::new();
        for row in self.storage.get_all(filter).await? {
            let Some(id) = id_of(&row.base.principal) else {
                continue;
            };
            let scope = scopes.entry(id).or_insert_with(|| Some(Vec::new()));
            match (row.site_id(), scope.as_mut()) {
                (None, _) => *scope = None,
                (Some(site_id), Some(listed)) => listed.push(site_id),
                (Some(_), None) => {}
            }
        }
        Ok(scopes)
    }

    /// The users assigned `site_id`, directly or through All sites.
    pub async fn user_ids_on_site(
        &self,
        site_id: Uuid,
        organization_id: Uuid,
    ) -> Result<Vec<Uuid>> {
        let rows = self
            .storage
            .get_for(&R::SITE_ASSIGNMENT_ORGANIZATION.at(organization_id))
            .await?;
        Ok(rows
            .iter()
            .filter(|row| match row.base.assignment.scope {
                AssignmentScope::AllSites => true,
                AssignmentScope::Site(id) => id == site_id,
            })
            .filter_map(|row| match row.base.principal {
                Principal::User(id) => Some(id),
                Principal::ApiKey(_) | Principal::Invite(_) => None,
            })
            .collect())
    }

    fn row(organization_id: Uuid, principal: Principal, assignment: Assignment) -> SiteAssignment {
        SiteAssignment::new(SiteAssignmentBase {
            organization_id,
            principal,
            assignment,
        })
    }

    /// A transaction for a principal's role and rows to commit together.
    pub(crate) async fn begin(&self) -> Result<Transaction<'static, Postgres>> {
        self.storage.begin_multi_table_transaction().await
    }

    /// Replace every row of `principal`, inside `tx`.
    pub(crate) async fn replace_in_tx(
        &self,
        organization_id: Uuid,
        principal: Principal,
        assignments: &[Assignment],
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<()> {
        let rows: Vec<SiteAssignment> = assignments
            .iter()
            .map(|assignment| Self::row(organization_id, principal, *assignment))
            .collect();
        self.storage
            .replace_for_in_tx(&principal.assignments(), &rows, tx)
            .await?;
        Ok(())
    }

    /// Add one row, keeping the rest.
    pub(crate) async fn add(
        &self,
        organization_id: Uuid,
        principal: Principal,
        assignment: Assignment,
    ) -> Result<()> {
        let row = Self::row(organization_id, principal, assignment);
        self.storage.add_for(&principal.assignments(), &row).await?;
        Ok(())
    }
}
