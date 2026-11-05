/**
 * The badge that says an entity is known only from another device's report and awaits a person.
 *
 * The frontend half of one rule: the backend's `EntitySource::confirmation` and
 * `StorableFilter::unconfirmed` decide the same thing from the same `source`, and the flag read
 * here is the `entity_sources` fixture's `unconfirmed`, so nothing in this file restates which
 * sources are unconfirmed. Hosts (an LLDP/CDP far end, a MAC a switch learned) and subnets (an
 * assumed range) share it, the way every entity shares `getFreshnessTag`, and it renders wherever
 * the Stale pill does.
 *
 * Violet, the Inferred source's colour, since an unconfirmed entity is exactly an inferred one; not
 * Stale's amber or a daemon's red: nothing is behind and nothing is broken; a guess awaits a person.
 */

import { CircleDashedCheck } from '@lucide/svelte';

import type { components } from '$lib/api/schema';
import type { TagProps } from '$lib/shared/components/data/types';
import { entitySources } from '$lib/shared/stores/metadata';
import { toColor } from '$lib/shared/utils/styling';
import {
	common_unconfirmed,
	hosts_unconfirmedViaForwardingTable,
	hosts_unconfirmedViaNeighbour,
	subnets_rangeAssumedDetail
} from '$lib/paraglide/messages';

type EntitySource = components['schemas']['EntitySource'];
type InferredVia = components['schemas']['InferredVia'];

/** Any host or subnet, or an older payload that predates `source`. */
export interface ConfirmationSubject {
	source?: EntitySource;
}

/** Whether this entity awaits confirmation. */
export function isUnconfirmed(entity: ConfirmationSubject): boolean {
	if (!entity.source) return false;
	const metadata = entitySources.getMetadata(entity.source.type) as { unconfirmed?: boolean };
	return metadata?.unconfirmed === true;
}

/** What reported an unconfirmed entity, or `null` for a confirmed one. */
export function unconfirmedVia(entity: ConfirmationSubject): InferredVia | null {
	if (!isUnconfirmed(entity) || !entity.source || !('via' in entity.source)) return null;
	return entity.source.via ?? 'Neighbour';
}

/** Why the entity is unconfirmed, as one sentence. */
export function unconfirmedReason(via: InferredVia): string {
	switch (via) {
		case 'Neighbour':
			return hosts_unconfirmedViaNeighbour();
		case 'ForwardingTable':
			return hosts_unconfirmedViaForwardingTable();
		case 'ReportedAddress':
			return subnets_rangeAssumedDetail();
	}
}

/** The Unconfirmed pill, or `null` when there is nothing to say. */
export function getConfirmationTag(entity: ConfirmationSubject): TagProps | null {
	const via = unconfirmedVia(entity);
	if (!via) return null;
	return {
		label: common_unconfirmed(),
		color: toColor('violet'),
		icon: CircleDashedCheck,
		pill: true,
		title: unconfirmedReason(via)
	};
}
