/**
 * How recently discovery observed an entity.
 *
 * This is the frontend half of one shared rule — the backend derives the same
 * verdict in `DiscoveryTracked::freshness` for the discovery digest email, and
 * the same predicate again in `StorableFilter::stale_by_network` for the
 * server-side "Stale only" filter. All three read the same two persisted
 * inputs (`last_seen_at` and the entity's network `stale_after_hours`), so a
 * host reported stale in the digest is the host badged stale in the app.
 * Change one, change all three.
 *
 * Every entity is judged on its OWN `last_seen_at`, with no inheritance from a
 * parent host. The digest applies a parent/child rule (`ChildPolicy`) because
 * it reports one scan's events, where "nothing was observed about this host's
 * children" is meaningful. A persistent badge answers a different question —
 * "when was this last seen?" — and inheriting there produced a node marked
 * Stale whose tooltip read "last seen 2 hours ago", and made topology disagree
 * with the inventory, which never inherited.
 */

import { Clock } from 'lucide-svelte';
import type { components } from '$lib/api/schema';
import type { Network } from '$lib/features/networks/types';
import type { CardFieldItem, TagProps } from '$lib/shared/components/data/types';
import type { EntityDiscriminants } from '$lib/api/entities';
import { entities } from '$lib/shared/stores/metadata';
import { toColor } from '$lib/shared/utils/styling';
import { formatDateNumeric, formatRelativeTime } from '$lib/shared/utils/formatting';
import {
	common_entityLastSeenAgo,
	common_lastSeenAgo,
	common_never,
	common_stale,
	common_staleWithDate,
	topology_neighborLastReportedAgo
} from '$lib/paraglide/messages';

/** Derived from the backend enum — never hand-maintain this union. */
export type EntityFreshness = components['schemas']['EntityFreshness'];

type EntitySource = components['schemas']['EntitySource'];

/** The subset of an entity freshness depends on. Hosts, services and subnets all satisfy it. */
export interface FreshnessSubject {
	last_seen_at?: string;
	source?: EntitySource;
}

/**
 * Discovery only refreshes `last_seen_at` on entities it created. A manual or
 * system entity's timestamp is frozen at creation, so judging it would mark
 * every hand-curated asset stale once it aged past the window.
 *
 * Absent `source` means "always discovery-created": IPAddress, Port, Interface
 * and Binding carry no source column because they cannot be created any other
 * way. This mirrors `DiscoveryTracked::is_discovery_managed`, whose default is
 * likewise `true` and which Host / Service / Subnet / Vlan override with their
 * own `source`. Treating an absent source as unmanaged would make those four
 * types permanently Current here while the backend judged them normally.
 */
function isDiscoveryManaged(source: EntitySource | undefined): boolean {
	if (source === undefined) return true;
	// The same set as `EntitySource::is_from_discovery` and the SQL guard in `stale_by_network`.
	// Inferred counts: the backend treats it as discovered for staleness, as the digest already does.
	return (
		source.type === 'Discovery' ||
		source.type === 'DiscoveryWithMatch' ||
		source.type === 'Inferred'
	);
}

/**
 * `Current` unless discovery manages this entity and hasn't observed it within
 * its network's window. Never returns `New` — that bucket exists only for the
 * digest's per-scan framing; the inventory surfaces `created_at` directly.
 *
 * The window comes from `effective_stale_after_hours`, which the server has
 * already resolved against its own default — the frontend deliberately holds no
 * default of its own, so it cannot drift from the digest. With no network in
 * hand we make no claim rather than guessing.
 */
export function entityFreshness(
	entity: FreshnessSubject,
	network: Network | undefined
): EntityFreshness {
	const windowHours = network?.effective_stale_after_hours;
	if (!windowHours || !entity.last_seen_at || !isDiscoveryManaged(entity.source)) return 'current';
	const cutoff = Date.now() - windowHours * 60 * 60 * 1000;
	return new Date(entity.last_seen_at).getTime() < cutoff ? 'stale' : 'current';
}

/**
 * How fresh the *evidence for a link* is, judged on the interface at one of its ends.
 *
 * An interface's own `last_seen_at` answers "was this port observed", which a port keeps
 * satisfying long after its neighbour record stops arriving — so a link whose evidence has
 * completely disappeared reads Current on both endpoints. `neighbor_seen_at` is the adjacency's
 * own subject, and it is judged here by exactly the rule and window above rather than a second
 * one: same `effective_stale_after_hours`, same vocabulary.
 *
 * `null`/absent `neighbor_seen_at` means no scan has ever carried evidence for this row — unknown,
 * never stale. `entityFreshness` already answers `current` for an absent timestamp, so rows
 * predating the column are safe without a special case here.
 */
export function neighborEvidenceFreshness(
	iface: { neighbor_seen_at?: string | null },
	network: Network | undefined
): EntityFreshness {
	return entityFreshness(neighborEvidenceSubject(iface), network);
}

/**
 * The freshness subject of the *adjacency* a row records, for the functions above.
 *
 * Shaping it as a `FreshnessSubject` is the point: `entityFreshness` and `getFreshnessTag` then
 * apply exactly the rule and render exactly the pill a stale host gets, so a link and an entity
 * can never disagree about what stale means or look different when they say it.
 */
export function neighborEvidenceSubject(iface: {
	neighbor_seen_at?: string | null;
}): FreshnessSubject {
	return { last_seen_at: iface.neighbor_seen_at ?? undefined };
}

/**
 * "Neighbor last reported 3d ago", the wording for an adjacency rather than an entity.
 *
 * Deliberately not `lastSeenLabel`: on a link, "last seen" reads as a claim about the port, which
 * is the exact confusion this column exists to end. The port was seen minutes ago; it is the
 * neighbour report that stopped arriving.
 */
export function neighborEvidenceLabel(iface: { neighbor_seen_at?: string | null }): string {
	if (!iface.neighbor_seen_at) return common_never();
	return topology_neighborLastReportedAgo({ time: formatRelativeTime(iface.neighbor_seen_at) });
}

/**
 * The stale pill for an adjacency: the shared amber tag, re-titled to name the neighbour report
 * as its subject.
 */
export function neighborEvidenceTag(
	iface: { neighbor_seen_at?: string | null },
	network: Network | undefined
): TagProps | null {
	const tag = getFreshnessTag(neighborEvidenceSubject(iface), network);
	return tag && { ...tag, title: neighborEvidenceLabel(iface) };
}

/**
 * Status tag for an entity card, or `null` when there is nothing to say.
 *
 * Amber rather than red, matching `getDaemonStatusTag`'s split: red means
 * broken (unreachable), amber means behind (outdated). A stale host may be
 * perfectly healthy and simply unobserved — so the badge must not read as an
 * error. The label carries the meaning without relying on colour.
 */
export function getFreshnessTag(
	entity: FreshnessSubject,
	network: Network | undefined,
	opts: {
		/**
		 * Display name of the entity's type ("IP Address", "Service", …), from
		 * `entities.getName()`. Names the thing the verdict is about, which
		 * matters where cards of different types sit side by side.
		 */
		entityTypeLabel?: string;
	} = {}
): TagProps | null {
	if (entityFreshness(entity, network) !== 'stale') return null;
	return {
		label: common_stale(),
		color: toColor('amber'),
		icon: Clock,
		title: lastSeenLabel(entity, opts.entityTypeLabel)
	};
}

/**
 * "IP Address last seen 12d ago" when the type is known, "Last seen 12d ago"
 * otherwise, or a never-observed fallback.
 */
export function lastSeenLabel(entity: FreshnessSubject, entityTypeLabel?: string): string {
	if (!entity.last_seen_at) return common_never();
	const time = formatRelativeTime(entity.last_seen_at);
	return entityTypeLabel
		? common_entityLastSeenAgo({ entity: entityTypeLabel, time })
		: common_lastSeenAgo({ time });
}

/**
 * Staleness chips for a "Last seen" field, on any entity discovery observes.
 *
 * Staleness is a qualifier on when something was last seen, not a status of its
 * own — a host, subnet, VLAN or service has no status. Given a Status column of
 * its own, `getFreshnessTag` returns a tag only for rows past their network's
 * window, so the column sat empty on every healthy row while the date beside it
 * said the same thing less precisely.
 *
 * So one column carries both: the plain date normally, and once a row has aged
 * out the same date inside an amber tag — "8/4/26 (Stale)" — so the column
 * reads consistently down its length and the date never disappears just because
 * a row went stale. Returning `undefined` (not `[]`) is what makes the cell fall
 * back to rendering the date.
 *
 * `entityType` names the thing the verdict is about in the tag's tooltip, since
 * these lists sit side by side.
 */
export function lastSeenItems<T extends FreshnessSubject & { network_id?: string | null }>(
	networks: () => Network[],
	entityType: EntityDiscriminants
): (entity: T) => CardFieldItem[] | undefined {
	return (entity) => {
		const network = networks().find((n) => n.id === entity.network_id);
		const tag = getFreshnessTag(entity, network, {
			entityTypeLabel: entities.getName(entityType) || undefined
		});
		if (!tag || !entity.last_seen_at) return undefined;
		return [
			{
				id: 'stale',
				label: common_staleWithDate({ date: formatDateNumeric(entity.last_seen_at) }),
				color: tag.color,
				icon: tag.icon,
				// The relative time ("last seen 45d ago") is the part the absolute
				// date doesn't give you at a glance.
				title: tag.title
			}
		];
	};
}
