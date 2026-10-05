//! Topologies

use super::*;

pub(super) fn generate_topologies(
    sites: &[Site],
    tags: &[Tag],
    now: DateTime<Utc>,
) -> Vec<Topology> {
    let critical_tag_id = tags
        .iter()
        .find(|t| t.base.name == "Critical")
        .map(|t| t.id);

    sites
        .iter()
        .map(|site| {
            let mut base = TopologyBase::new(site.id);
            // Add Critical tag to the default ByTag element rule
            if let Some(tag_id) = critical_tag_id {
                let mut request = TopologyRequestOptions::default();
                // Replace the default empty ByTag rule with one that includes Critical
                for rule in &mut request.element_rules {
                    if matches!(rule.rule, ElementRule::ByTag { .. }) {
                        rule.rule = ElementRule::ByTag {
                            tag_ids: vec![tag_id],
                            title: None,
                        };
                    }
                }
                base.options = TopologyOptions {
                    request,
                    ..base.options
                };
            }
            Topology {
                id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                base,
            }
        })
        .collect()
}
