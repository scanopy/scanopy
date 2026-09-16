//! ENTITY-MIB: the hardware inventory in entPhysicalTable.

use super::*;

/// Query ENTITY-MIB entPhysicalTable for hardware inventory.
/// Returns the best-match physical entity (chassis > stack > module).
pub async fn query_entity_physical<T: SnmpWalkTransport>(
    session: &mut T,
    ip: IpAddr,
) -> Result<SnmpCollection<Option<DeviceInventory>>> {
    struct PhysicalEntry {
        description: Option<String>,
        class: Option<i32>,
        name: Option<String>,
        serial_number: Option<String>,
        manufacturer: Option<String>,
        model: Option<String>,
        firmware_revision: Option<String>,
        software_revision: Option<String>,
    }

    let mut entries: HashMap<i32, PhysicalEntry> = HashMap::new();
    let mut shortfall = Shortfall::default();

    let columns = [
        (oids::entity::entry::ENT_PHYSICAL_DESCR, "descr"),
        (oids::entity::entry::ENT_PHYSICAL_CLASS, "class"),
        (oids::entity::entry::ENT_PHYSICAL_NAME, "name"),
        (oids::entity::entry::ENT_PHYSICAL_SERIAL_NUM, "serialNum"),
        (oids::entity::entry::ENT_PHYSICAL_MFG_NAME, "mfgName"),
        (oids::entity::entry::ENT_PHYSICAL_MODEL_NAME, "modelName"),
        (
            oids::entity::entry::ENT_PHYSICAL_FIRMWARE_REV,
            "firmwareRev",
        ),
        (
            oids::entity::entry::ENT_PHYSICAL_SOFTWARE_REV,
            "softwareRev",
        ),
    ];

    for (base_oid_str, column_name) in columns {
        // OID suffix is entPhysicalIndex (single integer).
        walk_column(
            session,
            ip,
            base_oid_str,
            &mut shortfall,
            |suffix, value| {
                let Some(&index_u64) = suffix.last() else {
                    return;
                };
                let entry = entries
                    .entry(index_u64 as i32)
                    .or_insert_with(|| PhysicalEntry {
                        description: None,
                        class: None,
                        name: None,
                        serial_number: None,
                        manufacturer: None,
                        model: None,
                        firmware_revision: None,
                        software_revision: None,
                    });
                match column_name {
                    "descr" => entry.description = value_to_string(value),
                    "class" => entry.class = value_to_i32(value),
                    "name" => entry.name = value_to_string(value),
                    "serialNum" => {
                        entry.serial_number = value_to_string(value).filter(|s| !s.is_empty())
                    }
                    "mfgName" => {
                        entry.manufacturer = value_to_string(value).filter(|s| !s.is_empty())
                    }
                    "modelName" => entry.model = value_to_string(value).filter(|s| !s.is_empty()),
                    "firmwareRev" => {
                        entry.firmware_revision = value_to_string(value).filter(|s| !s.is_empty())
                    }
                    "softwareRev" => {
                        entry.software_revision = value_to_string(value).filter(|s| !s.is_empty())
                    }
                    _ => {}
                }
            },
        )
        .await;
    }

    // Select best match: prefer chassis (3), fallback to stack (11), then module (9); within the
    // strongest class present, the lowest `entPhysicalIndex`.
    //
    // The index tiebreak is what makes this deterministic, and it is load-bearing rather than
    // decorative. This used to be `entries.values().find(..)`, and `entries` is a `HashMap` whose
    // iteration order is randomised per instance — so a device serving more than one chassis row,
    // which is any switch stack, returned a different row on each call. `model` is refreshable, so
    // a stack whose members report different model strings already rewrote `hosts.model` on an
    // arbitrary subset of scans; the two revisions read below are refreshable for the same reason
    // and would have flapped the same way.
    let best = [3, 11, 9].into_iter().find_map(|class| {
        entries
            .iter()
            .filter(|(_, e)| e.class == Some(class))
            .min_by_key(|&(index, _)| *index)
            .map(|(_, e)| e)
    });

    // Every field comes from the one selected row. A chassis is a single physical thing, so a
    // serial taken from one row and a firmware revision from another would describe no device
    // that exists.
    let result = best.map(|e| DeviceInventory {
        description: e.description.clone().or_else(|| e.name.clone()),
        manufacturer: e.manufacturer.clone(),
        model: e.model.clone(),
        serial_number: e.serial_number.clone(),
        firmware_revision: e.firmware_revision.clone(),
        software_revision: e.software_revision.clone(),
    });

    debug!(
        "ENTITY-MIB query from {} returned {} physical entries, best match: {}",
        ip,
        entries.len(),
        result.is_some()
    );

    Ok(SnmpCollection::from_walk(result, shortfall))
}
