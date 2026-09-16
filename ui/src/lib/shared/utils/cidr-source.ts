/**
 * The badge that says a subnet's range was assumed rather than read.
 *
 * One definition, rendered by the shared `Tag` wherever a subnet appears — the subnet list, the
 * topology inspector, a picker — for the same reason `getFreshnessTag` is one definition: a subnet
 * and the same subnet drawn somewhere else must not disagree about how much it is trusted, or look
 * different when they say it.
 *
 * Colour is deliberately neither of the two already spoken for. `getFreshnessTag` owns amber
 * ("behind"), `getDaemonStatusTag` owns red ("broken"), and this is neither: the range is probably
 * right and nothing is wrong — an operator is being asked to confirm a guess. As with those, the
 * label carries the meaning without relying on colour.
 */

import { CloudAlert } from 'lucide-svelte';

import attributeMethods from '$lib/data/attribute-methods.json';
import type { CardFieldItem, TagProps } from '$lib/shared/components/data/types';
import { sourceKey, type AttributeSource } from '$lib/shared/utils/attribute-source';
import {
	subnets_rangeAssumed,
	subnets_rangeAssumedDetail,
	subnets_rangeAssumedWithCidr
} from '$lib/paraglide/messages';
import { toColor } from '$lib/shared/utils/styling';

/**
 * The shape these read: any subnet, and any older payload that predates the column.
 *
 * Optional rather than required so a historical scan record or a cached response without it reads
 * as settled instead of throwing — an absent source is not a guess.
 */
type WithCidrSource = { cidr_source?: AttributeSource };

/**
 * Every source that sits at the `Inferred` tier, as the backend groups them.
 *
 * Read from the metadata fixture rather than matched against a variant name here. The tier a source
 * belongs to is a backend decision — it is the same `AttributeSource::method()` the applier orders
 * by — and the previous version of this file hardcoded `=== 'Inferred'`, which meant a source added
 * at that tier silently stopped raising the badge. Keyed on the whole source, probe included,
 * because `Probe(Snmp)` and `Probe(Docker)` sit at different tiers.
 */
const INFERRED_SOURCES: ReadonlySet<string> = new Set(
	(
		(attributeMethods.find((method) => method.id === 'Inferred')?.metadata?.sources ??
			[]) as AttributeSource[]
	).map(sourceKey)
);

/** Whether this subnet's range is a guess awaiting confirmation. */
export function isProvisionalCidr(subnet: WithCidrSource): boolean {
	return subnet.cidr_source ? INFERRED_SOURCES.has(sourceKey(subnet.cidr_source)) : false;
}

/**
 * The provisional-range tag, or `null` when there is nothing to say.
 *
 * `null` for everything above the `Inferred` tier: a range read off a device and one a person typed
 * are both settled, and the badge exists only to mark the third case.
 */
export function getCidrSourceTag(subnet: WithCidrSource): TagProps | null {
	if (!isProvisionalCidr(subnet)) return null;
	return {
		label: subnets_rangeAssumed(),
		color: toColor('indigo'),
		icon: CloudAlert,
		title: subnets_rangeAssumedDetail()
	};
}

/**
 * The badge as a data-table field item, for the `cidr` column.
 *
 * Sits beside the range rather than in a column of its own: the claim is about *that value*, and a
 * separate column would read as an attribute of the subnet instead of a caveat on its CIDR.
 *
 * **The CIDR goes inside the label**, the way `lastSeenItems` puts the date inside its stale tag.
 * A cell renders chips *or* its plain value, never both — returning a chip here replaces the
 * column's value — so a badge that did not carry the CIDR would leave the row saying "Range
 * assumed" and never saying which range. Returning `undefined` (not `[]`) is what makes a settled
 * row fall back to the plain value.
 */
export function cidrSourceItems<T extends WithCidrSource & { cidr?: string }>(): (
	subnet: T
) => CardFieldItem[] | undefined {
	return (subnet) => {
		const tag = getCidrSourceTag(subnet);
		if (!tag || !subnet.cidr) return undefined;
		return [
			{
				id: 'inferred-cidr',
				label: subnets_rangeAssumedWithCidr({ cidr: subnet.cidr }),
				color: tag.color,
				icon: tag.icon,
				title: tag.title
			}
		];
	};
}
