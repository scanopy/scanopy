use uuid::Uuid;

use super::{Email, EmailCategory, EmailPreference, PausableCategory, links};
use crate::server::{
    digest::payload::{
        AffectedHostCard, DiscoveryDigestPayload, EntityFreshness, InterfaceSummary,
        IpAddressSummary, PortSummary, ServiceSummary, SubnetSummary, VlanSummary,
    },
    shared::{
        concepts::Concept,
        entities::EntityDiscriminants,
        types::{Color, metadata::EntityMetadataProvider},
    },
};

/// Per-discovery-session digest summarising hosts added/changed/vanished,
/// VLANs, and subnets scanned. Holds the computed payload plus the installed
/// app's `public_url`, used to build absolute deep-links into the UI.
pub struct DiscoveryDigest<'a> {
    pub payload: &'a DiscoveryDigestPayload,
    pub base_url: &'a str,
}

impl Email for DiscoveryDigest<'_> {
    fn subject(&self) -> String {
        format!("{}: {}", TITLE, self.payload.network_name)
    }

    fn category(&self) -> EmailCategory {
        EmailCategory::Digest
    }

    fn preference(&self) -> EmailPreference {
        EmailPreference::Pausable(PausableCategory::DiscoveryDigest)
    }

    fn campaign(&self) -> &'static str {
        "discovery_digest"
    }

    fn body_html(&self) -> String {
        let payload = self.payload;
        let started = payload
            .started_at
            .format("%b %-d, %Y %H:%M UTC")
            .to_string();
        let finished = payload
            .finished_at
            .format("%b %-d, %Y %H:%M UTC")
            .to_string();
        let base = self.base_url.trim_end_matches('/');
        // Carries `{base_url}` / `{utm}`, which `render_html` expands. The
        // entity links below are built per id, so they resolve `base` here.
        let settings_url = links::SETTINGS_EMAIL;

        let summary_section = render_summary_banner(payload);
        let legend_section = render_legend(payload.stale_after_hours);
        let subnets_section = render_subnets_section(&payload.subnets_scanned, base, self);
        let hosts_added_section =
            render_host_cards_section("New hosts discovered", &payload.hosts_added, base, self);
        let hosts_stale_section =
            render_host_cards_section("Stale hosts", &payload.hosts_stale, base, self);
        let hosts_changed_section =
            render_host_cards_section("Hosts with changes", &payload.hosts_changed, base, self);
        let vlans_added_section = render_vlan_list_section("VLANs detected", &payload.vlans_added);
        let vlans_stale_section = render_vlan_list_section("Stale VLANs", &payload.vlans_stale);

        BODY.replace("{network_name}", &html_escape(&payload.network_name))
            .replace("{started_at}", &started)
            .replace("{finished_at}", &finished)
            .replace("{settings_url}", settings_url)
            .replace("{summary_section}", &summary_section)
            .replace("{legend_section}", &legend_section)
            .replace("{subnets_section}", &subnets_section)
            .replace("{hosts_added_section}", &hosts_added_section)
            .replace("{hosts_stale_section}", &hosts_stale_section)
            .replace("{hosts_changed_section}", &hosts_changed_section)
            .replace("{vlans_added_section}", &vlans_added_section)
            .replace("{vlans_stale_section}", &vlans_stale_section)
    }
}

const TITLE: &str = "Discovery scan summary";

const BODY: &str = r#"                    <!-- Main Content -->
                    <tr>
                        <td style="padding: 0 40px 20px 40px;">
                            <h1 style="margin: 0 0 20px 0; font-size: 24px; font-weight: 600; color: #1a1a1a; text-align: center;">Discovery scan summary</h1>
                            <p style="margin: 0 0 8px 0; font-size: 16px; line-height: 24px; color: #4a4a4a;">Network: <strong>{network_name}</strong></p>
                            <p style="margin: 0 0 20px 0; font-size: 14px; line-height: 20px; color: #6b7280;">Scan ran {started_at} → {finished_at}.</p>
                            {summary_section}
                            {legend_section}
                            {subnets_section}
                            {hosts_added_section}
                            {hosts_stale_section}
                            {hosts_changed_section}
                            {vlans_added_section}
                            {vlans_stale_section}
                        </td>
                    </tr>

                    <!-- Settings Link -->
                    <tr>
                        <td style="padding: 0 40px 30px 40px; border-top: 1px solid #e5e7eb;">
                            <p style="margin: 20px 0 0 0; font-size: 13px; line-height: 18px; color: #9ca3af;">You're receiving this because you have access to {network_name} on Scanopy. <a href="{settings_url}" style="color: #2563eb;">Manage email preferences</a>.</p>
                        </td>
                    </tr>
"#;

/// Inline tag bags expand at 10 items. Host-card sections expand at 5 —
/// recipients only want a peek at each host's footprint by default.
const MAX_TAG_ITEMS_INLINE: usize = 10;
const MAX_HOST_CARDS_INLINE: usize = 5;

/// Carrier for a single tag chip — entity colour stays bound to the
/// discriminant; status is encoded with glyph + strikethrough, never with
/// colour.
struct TagItem {
    label: String,
    color: Color,
    status: EntityFreshness,
    href: Option<String>,
    /// Already-absolute URL (relative `/logos/...` paths rewritten by the
    /// caller against `public_url`).
    logo_url: Option<String>,
}

/// Render a single tag chip. Background/text colour come from the entity
/// type. Status is conveyed by the prefix glyph + (for Missing) strike-
/// through on the label. When `href` is `Some` the entire chip becomes a
/// clickable anchor that opens the corresponding modal in the app.
fn render_tag(tag: &TagItem) -> String {
    let (bg, fg) = tag.color.email_tag_hex();
    // No strikethrough for stale: it reads as "removed", and the only claim we
    // make is "not observed within this network's window". Italic carries the
    // softer meaning and survives every mail client.
    let (prefix, label_style) = match tag.status {
        EntityFreshness::New => ("+ ", ""),
        EntityFreshness::Stale => ("◷ ", "font-style: italic;"),
        EntityFreshness::Current => ("", ""),
    };
    let logo = tag.logo_url.as_deref().filter(|u| !u.is_empty()).map_or_else(
        String::new,
        |u| {
            format!(
                r#"<img src="{src}" alt="" width="14" height="14" style="vertical-align: middle; margin-right: 4px; border: 0;">"#,
                src = html_escape(u),
            )
        },
    );
    let chip = format!(
        r#"<span style="display: inline-block; padding: 3px 10px; margin: 2px 4px 2px 0; border-radius: 12px; background-color: {bg}; color: {fg}; font-size: 13px; line-height: 1.4; white-space: nowrap;">{prefix}{logo}<span style="{label_style}">{label}</span></span>"#,
        bg = bg,
        fg = fg,
        prefix = prefix,
        logo = logo,
        label_style = label_style,
        label = html_escape(&tag.label),
    );
    match &tag.href {
        Some(href) if !href.is_empty() => format!(
            r#"<a href="{href}" style="text-decoration: none; color: inherit;">{chip}</a>"#,
            href = html_escape(href),
            chip = chip,
        ),
        _ => chip,
    }
}

/// Wrap-inline tag bag. Up to `MAX_TAG_ITEMS_INLINE` tags render visible;
/// remaining items live inside a `<details>` whose `<summary>` reads
/// "Show N more".
fn render_tag_bag(tags: &[TagItem]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    let visible: String = tags
        .iter()
        .take(MAX_TAG_ITEMS_INLINE)
        .map(render_tag)
        .collect();
    if tags.len() <= MAX_TAG_ITEMS_INLINE {
        return visible;
    }
    let hidden: String = tags
        .iter()
        .skip(MAX_TAG_ITEMS_INLINE)
        .map(render_tag)
        .collect();
    let more = tags.len() - MAX_TAG_ITEMS_INLINE;
    format!(
        r#"{visible}<details style="margin: 4px 0 0 0;"><summary style="cursor: pointer; font-size: 12px; color: #2563eb;">Show {more} more</summary><div style="margin-top: 6px;">{hidden}</div></details>"#,
        visible = visible,
        more = more,
        hidden = hidden,
    )
}

/// Field row inside a host card: bold label + inline tag bag below it.
/// Hidden entirely when the tag bag is empty.
fn render_tag_row(label: &str, tags: &[TagItem]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    format!(
        r#"<div style="margin: 0 0 10px 0; font-size: 13px;"><div style="font-weight: 600; color: #4b5563; margin: 0 0 4px 0;">{label}</div><div>{tags}</div></div>"#,
        label = html_escape(label),
        tags = render_tag_bag(tags),
    )
}

fn render_section(heading: &str, body_html: &str) -> String {
    format!(
        r#"<h2 style="margin: 24px 0 8px 0; font-size: 16px; font-weight: 600; color: #1a1a1a;">{}</h2>{}"#,
        html_escape(heading),
        body_html,
    )
}

/// Glyph legend explaining the per-tag status encoding. No colour — colour
/// stays bound to the entity type. Placed at the top of the body just
/// below the summary banner.
///
/// The staleness window is per-network, so the legend states this network's
/// effective value rather than a fixed rule. Wording matches the app's badge
/// ("Stale") so the same entity reads the same way in both places.
fn render_legend(stale_after_hours: i64) -> String {
    let window = humanize_hours(stale_after_hours);
    format!(
        r#"<div style="margin: 0 0 16px 0; padding: 10px 14px; background-color: #f9fafb; border-radius: 6px; font-size: 12px; color: #4b5563; line-height: 1.5;"><div style="margin: 0 0 6px 0;"><span style="margin-right: 14px;">(<strong>+</strong> new)</span><span style="margin-right: 14px;">(current)</span><span>(<strong>◷</strong> <em>stale</em>)</span></div><div style="font-size: 12px; color: #6b7280;"><strong>◷</strong> means discovery hasn't observed this entity for {window}, this network's staleness window. It doesn't mean the entity was removed — it may simply be powered off or out of reach. Entities this scan didn't cover aren't listed at all.</div></div>"#,
    )
}

/// "672" → "28 days". Whole days when it divides evenly, hours otherwise.
fn humanize_hours(hours: i64) -> String {
    let plural = |n: i64, unit: &str| {
        if n == 1 {
            format!("1 {unit}")
        } else {
            format!("{n} {unit}s")
        }
    };
    if hours >= 24 && hours % 24 == 0 {
        plural(hours / 24, "day")
    } else {
        plural(hours, "hour")
    }
}

fn render_subnets_section(subnets: &[SubnetSummary], base: &str, email: &dyn Email) -> String {
    if subnets.is_empty() {
        return String::new();
    }
    let color = EntityDiscriminants::Subnet.color();
    let tags: Vec<TagItem> = subnets
        .iter()
        .map(|s| TagItem {
            label: s.label.clone(),
            color,
            status: Default::default(),
            href: Some(email.with_utm(&format!("{base}/?modal=subnet-editor&id={}", s.id))),
            logo_url: None,
        })
        .collect();
    let header = format!("Subnets scanned ({})", subnets.len());
    render_section(&header, &render_tag_bag(&tags))
}

fn render_vlan_list_section(heading: &str, vlans: &[VlanSummary]) -> String {
    if vlans.is_empty() {
        return String::new();
    }
    let color = EntityDiscriminants::Vlan.color();
    let tags: Vec<TagItem> = vlans
        .iter()
        .map(|v| {
            let label = if v.name.is_empty() {
                format!("VLAN {}", v.vlan_number)
            } else {
                format!("VLAN {} — {}", v.vlan_number, v.name)
            };
            TagItem {
                label,
                color,
                status: Default::default(),
                href: None, // VLANs aren't deep-linkable in the UI
                logo_url: None,
            }
        })
        .collect();
    let header = format!("{} ({})", heading, vlans.len());
    render_section(&header, &render_tag_bag(&tags))
}

/// Stats banner at the top of the digest body.
fn render_summary_banner(payload: &DiscoveryDigestPayload) -> String {
    let cells: Vec<(usize, &str)> = vec![
        (payload.hosts_added.len(), "new hosts"),
        (payload.hosts_stale.len(), "stale hosts"),
        (payload.hosts_changed.len(), "changed hosts"),
        (payload.vlans_added.len(), "VLANs detected"),
        (payload.vlans_stale.len(), "stale VLANs"),
        (payload.subnets_scanned.len(), "subnets scanned"),
    ];
    let inner: String = cells
        .iter()
        .map(|(count, label)| {
            format!(
                r#"<td style="padding: 8px 12px; vertical-align: top;"><div style="font-size: 22px; font-weight: 700; color: #1a1a1a; line-height: 1.2;">{}</div><div style="font-size: 12px; color: #6b7280;">{}</div></td>"#,
                count,
                html_escape(label),
            )
        })
        .collect();
    format!(
        r#"<table role="presentation" style="width: 100%; border-collapse: collapse; margin: 16px 0; background-color: #f9fafb; border-radius: 6px;"><tr>{}</tr></table>"#,
        inner,
    )
}

/// Render one section of host cards. Sections with more than 5 cards put
/// the overflow inside `<details>` so recipients can opt-in to the full
/// list. Summary reads "Show {N} more hosts".
fn render_host_cards_section(
    heading: &str,
    cards: &[AffectedHostCard],
    base: &str,
    email: &dyn Email,
) -> String {
    if cards.is_empty() {
        return String::new();
    }
    let header = format!("{} ({})", heading, cards.len());
    let visible: String = cards
        .iter()
        .take(MAX_HOST_CARDS_INLINE)
        .map(|c| render_host_card(c, base, email))
        .collect();
    if cards.len() <= MAX_HOST_CARDS_INLINE {
        return render_section(&header, &visible);
    }
    let hidden: String = cards
        .iter()
        .skip(MAX_HOST_CARDS_INLINE)
        .map(|c| render_host_card(c, base, email))
        .collect();
    let more = cards.len() - MAX_HOST_CARDS_INLINE;
    let inner = format!(
        r#"{visible}<details style="margin: 16px 0;"><summary style="cursor: pointer; display: inline-block; padding: 8px 16px; background-color: #ffffff; color: #2563eb; font-size: 13px; font-weight: 600; border: 1px solid #2563eb; border-radius: 6px; list-style: none;">Show {more} more hosts &#9662;</summary><div style="margin-top: 12px;">{hidden}</div></details>"#,
        visible = visible,
        more = more,
        hidden = hidden,
    );
    render_section(&header, &inner)
}

fn render_host_card(card: &AffectedHostCard, base: &str, email: &dyn Email) -> String {
    // Badge mirrors the per-tag glyph convention: a host that's listed
    // because its children changed has `Unchanged` status — the surrounding
    // section header carries the context, so no badge is shown.
    // Amber, not red: stale means "behind", not "broken" — the same split the
    // app's daemon status tags use (red = unreachable, amber = outdated). The
    // word carries the meaning unaided so the badge survives an image-blocking
    // client with no colour perception required.
    let badge = match card.status {
        EntityFreshness::New => Some(("+", "New", "#dcfce7", "#166534", "")),
        EntityFreshness::Stale => Some(("◷", "Stale", "#fef9c3", "#854d0e", "")),
        EntityFreshness::Current => None,
    }
    .map(|(glyph, title, bg, fg, extra)| {
        format!(
            r#"<span title="{title}" style="display: inline-block; min-width: 18px; padding: 2px 6px; font-size: 13px; font-weight: 700; text-align: center; border-radius: 999px; background-color: {bg}; color: {fg}; {extra}">{glyph}</span>"#,
        )
    })
    .unwrap_or_default();
    let host_href = email.with_utm(&format!("{base}/?modal=host-editor&id={}", card.host.id));
    let host_link = format!(
        r#"<a href="{href}" style="text-decoration: none; color: #1a1a1a;">{label}</a>"#,
        href = html_escape(&host_href),
        label = html_escape(&card.host.label),
    );

    // Split services into the non-container set + the Containers set,
    // mirroring HostCard.svelte. The Containers row only renders when the
    // host has at least one container service.
    let (containers, services): (Vec<_>, Vec<_>) =
        card.services.iter().partition(|s| s.is_container);

    let services_tags = tags_services(&services, card.host.id, base, email);
    let containerized_tags = tags_containerized_services(&containers, card.host.id, base, email);
    let ip_tags = tags_ips(&card.ip_addresses, card.host.id, base, email);
    let interface_tags = tags_interfaces(&card.interfaces, card.host.id, base, email);
    let port_tags = tags_ports(&card.ports, card.host.id, base, email);

    let mut rows = String::new();
    rows.push_str(&render_tag_row("Services", &services_tags));
    rows.push_str(&render_tag_row(
        "Containerized Services",
        &containerized_tags,
    ));
    rows.push_str(&render_tag_row("IP Addresses", &ip_tags));
    rows.push_str(&render_tag_row("Interfaces", &interface_tags));
    rows.push_str(&render_tag_row("Ports", &port_tags));

    format!(
        r#"<div style="margin: 0 0 16px 0; padding: 14px; background-color: #ffffff; border: 1px solid #e5e7eb; border-radius: 8px;"><div style="display: flex; align-items: center; justify-content: space-between; margin: 0 0 10px 0;"><div style="font-size: 16px; font-weight: 600;">{host_link}</div>{badge}</div>{rows}</div>"#,
        host_link = host_link,
        badge = badge,
        rows = rows,
    )
}

/// Rewrite a relative `/logos/...` path to an absolute URL using the
/// installed app's `public_url`. Absolute URLs pass through.
fn absolute_logo(raw: &Option<String>, base: &str) -> Option<String> {
    let raw = raw.as_deref()?;
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.to_string())
    } else if let Some(path) = raw.strip_prefix('/') {
        Some(format!("{base}/{path}"))
    } else {
        Some(raw.to_string())
    }
}

fn tags_ports(items: &[PortSummary], host_id: Uuid, base: &str, email: &dyn Email) -> Vec<TagItem> {
    let color = EntityDiscriminants::Port.color();
    items
        .iter()
        .map(|p| TagItem {
            label: p.label.clone(),
            color,
            status: p.status,
            href: Some(email.with_utm(&format!(
                "{base}/?modal=host-editor&id={host_id}&tab=ports&subEntityId={}",
                p.id
            ))),
            logo_url: None,
        })
        .collect()
}

fn tags_services(
    items: &[&ServiceSummary],
    _host_id: Uuid,
    base: &str,
    email: &dyn Email,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::Service.color();
    items
        .iter()
        .map(|s| TagItem {
            label: s.name.clone(),
            color,
            status: s.status,
            href: Some(email.with_utm(&format!("{base}/?modal=service-editor&id={}", s.id))),
            logo_url: absolute_logo(&s.logo_url, base),
        })
        .collect()
}

/// Containerised services get the UI's container concept colour
/// (`Concept::Virtualization`) so they read as a distinct concept on the
/// card without losing their service-ness.
fn tags_containerized_services(
    items: &[&ServiceSummary],
    _host_id: Uuid,
    base: &str,
    email: &dyn Email,
) -> Vec<TagItem> {
    let color = Concept::Virtualization.color();
    items
        .iter()
        .map(|s| TagItem {
            label: s.name.clone(),
            color,
            status: s.status,
            href: Some(email.with_utm(&format!("{base}/?modal=service-editor&id={}", s.id))),
            logo_url: absolute_logo(&s.logo_url, base),
        })
        .collect()
}

fn tags_ips(
    items: &[IpAddressSummary],
    host_id: Uuid,
    base: &str,
    email: &dyn Email,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::IPAddress.color();
    items
        .iter()
        .map(|ip| TagItem {
            label: ip.address.clone(),
            color,
            status: ip.status,
            href: Some(email.with_utm(&format!(
                "{base}/?modal=host-editor&id={host_id}&tab=ip-addresses&subEntityId={}",
                ip.id
            ))),
            logo_url: None,
        })
        .collect()
}

fn tags_interfaces(
    items: &[InterfaceSummary],
    host_id: Uuid,
    base: &str,
    email: &dyn Email,
) -> Vec<TagItem> {
    let color = EntityDiscriminants::Interface.color();
    items
        .iter()
        .map(|i| TagItem {
            label: i.label.clone(),
            color,
            status: i.status,
            href: Some(email.with_utm(&format!(
                "{base}/?modal=host-editor&id={host_id}&tab=interfaces&subEntityId={}",
                i.id
            ))),
            logo_url: None,
        })
        .collect()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
