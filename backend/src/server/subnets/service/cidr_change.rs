//! A person moving a subnet to a new range.
//!
//! Create handles a new range arriving next to the old ones: it dedups, corrects an assumed range
//! in place, and re-files what a narrowing displaces. A range moved through update used to get
//! none of that. It was written in place, with no duplicate check, no history and no re-filing, so
//! an edit could leave two rows with one range, or addresses filed in a subnet that no longer held
//! them most specifically. This module gives an edit the same treatment as create, plus history.
use super::*;
use crate::server::shared::services::traits::SnapshotMutator;

/// Why a range can't be moved where a person asked. Each maps to its own API error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CidrChangeRefusal {
    /// `0.0.0.0/0`, which belongs to the Internet and Remote Network rows.
    Organizational,
    /// Another live subnet on the site already has the range. Carries its name.
    Duplicate(String),
    /// An address the subnet holds would end up in no subnet. Carries the address.
    StrandsAddress(IpAddr),
}

impl SubnetService {
    /// Whether `current` can move to `new_cidr`, asked before anything is written.
    ///
    /// An assumed range with the same CIDR isn't a duplicate: the move absorbs it (see
    /// [`Self::absorb_assumed_ranges`]), the same way a reading corrects one on create.
    pub async fn check_cidr_change(
        &self,
        current: &Subnet,
        new_cidr: IpCidr,
    ) -> Result<Result<(), CidrChangeRefusal>> {
        let mut moved = current.clone();
        moved.base.cidr = SubnetCidr::new(SubnetCidrValue(new_cidr), AttributeSource::Manual);
        if moved.is_organizational_subnet() {
            return Ok(Err(CidrChangeRefusal::Organizational));
        }

        let live = self
            .storage
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[current.base.site_id]).live())
            .await?;

        // Container bridges are host-scoped and hold only what their runtime files in them, so a
        // LAN range that happens to equal one doesn't make placement ambiguous.
        if let Some(duplicate) = live.iter().find(|other| {
            other.id != current.id
                && other.base.cidr.value() == moved.base.cidr.value()
                && !other.is_organizational_subnet()
                && other.base.virtualization_service_id.is_none()
                && !other.base.source.confirmation().is_unconfirmed()
        }) {
            return Ok(Err(CidrChangeRefusal::Duplicate(
                duplicate.base.name().to_string(),
            )));
        }

        if let Some(ip) = self.stranded_by_narrowing(current, new_cidr, &live).await {
            return Ok(Err(CidrChangeRefusal::StrandsAddress(ip)));
        }
        Ok(Ok(()))
    }

    /// Write `entity`, whose range differs from `current`'s, and re-file every address the move
    /// affects.
    ///
    /// The old range is kept as a closed historical row, as a tag rename keeps the old name, so
    /// an as-of read before the move shows the range the subnet had then. The caller has run
    /// [`Self::check_cidr_change`].
    pub(super) async fn move_range(
        &self,
        current: Subnet,
        entity: &mut Subnet,
        authentication: AuthenticatedEntity,
        reason: UpdateReason,
    ) -> Result<Subnet> {
        let mut updated = self.close_and_clone(entity.clone()).await?;

        if let Some(org_id) = authentication.organization_id() {
            self.entity_tag_service
                .set_tags(
                    updated.id,
                    EntityDiscriminants::Subnet,
                    entity.base.tags.clone(),
                    org_id,
                )
                .await?;
        }
        updated.base.tags = entity.base.tags.clone();
        updated.base.vlan_ids = current.base.vlan_ids.clone();

        self.publish_updated(&current, &updated, authentication.clone(), reason)
            .await;

        // Narrowing first: what the move displaced goes wherever it now belongs.
        self.report_correction(
            &updated,
            (*current.base.cidr, current.base.cidr.source()),
            authentication.clone(),
        )
        .await;
        self.absorb_assumed_ranges(&updated, authentication).await?;
        self.rehome_into(&updated).await?;

        Ok(updated)
    }

    /// Merge every assumed range that `holder` now contains into it.
    ///
    /// A person's range outranks a guess, so a guess inside it was the same segment all along.
    /// Discovery leaves several nested guesses for a person to merge because merging deletes rows
    /// (see `create`); a person asserting the covering range is that person.
    pub(super) async fn absorb_assumed_ranges(
        &self,
        holder: &Subnet,
        authentication: AuthenticatedEntity,
    ) -> Result<()> {
        let live = self
            .storage
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[holder.base.site_id]).live())
            .await?;
        for assumed in live.iter().filter(|s| {
            s.id != holder.id
                && s.base.source.confirmation().is_unconfirmed()
                && holder.base.cidr.contains(&s.base.cidr.first_address())
                && holder.base.cidr.network_length() <= s.base.cidr.network_length()
        }) {
            self.merge_into(assumed.id, holder.id, authentication.clone())
                .await?;
        }
        Ok(())
    }

    /// Move onto `holder` the addresses that a wider subnet holds but `holder` now holds more
    /// specifically.
    ///
    /// Addresses are filed in the most specific subnet that contains them
    /// ([`placeable_subnet`]), and `needs_placement` re-files one only when its subnet stops
    /// containing it, which a wider subnet never does. Without this, moving a range inside a wider
    /// one would leave it empty and every address in the wider one. Container bridges and the
    /// organizational rows are skipped: neither nests, so neither holds an address that belongs in
    /// `holder`.
    async fn rehome_into(&self, holder: &Subnet) -> Result<()> {
        let live = self
            .storage
            .get_all(StorableFilter::<Subnet>::new_from_site_ids(&[holder.base.site_id]).live())
            .await?;
        for wider in live.iter().filter(|s| {
            s.id != holder.id
                && !s.is_organizational_subnet()
                && s.base.virtualization_service_id.is_none()
                && s.base.cidr.contains(&holder.base.cidr.first_address())
                && s.base.cidr.network_length() < holder.base.cidr.network_length()
        }) {
            for mut address in self.ip_address_service.get_for_subnet(&wider.id).await? {
                let ip = address.base.ip_address;
                if placeable_subnet(&live, ip).map(|s| s.id) != Some(holder.id) {
                    continue;
                }
                address.base.subnet_id = holder.id;
                self.ip_address_service
                    .update(
                        &mut address,
                        AuthenticatedEntity::System,
                        UpdateReason::IpAddressSubnetWidened,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}
