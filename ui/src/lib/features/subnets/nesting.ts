/**
 * Which subnet sits inside which, read from the server's `parent_subnet_id`.
 *
 * The server sends each subnet's parent only: children are the subnets that name it, so there is
 * one source and nothing to disagree. A parent missing from `subnets` (filtered out, or a synthetic
 * row the list hides) leaves its child at the top.
 */
import type { CardFieldItem } from '#lib/shared/components/data/types.ts';
import { isProvisionalCidr } from '#lib/shared/utils/cidr-source.ts';
import { subnets_containsCount, subnets_containsDetail } from '#lib/paraglide/messages.js';
import type { SubnetResponse } from './types/base';

/** Share of a subnet's usable addresses in use, 0 to 1. */
export function utilizationRatio(subnet: SubnetResponse): number {
	if (subnet.usable_addresses <= 0) return 0;
	return Math.min(subnet.used_addresses / subnet.usable_addresses, 1);
}

/**
 * The label that says a subnet holds others: "Contains N".
 *
 * Neutral, because nesting is information rather than a fault: a range a person recorded for a
 * whole site above the segments discovery read is deliberate. Empty on a provisional row, which
 * already carries the "Range assumed" badge and its own way to resolve a covering range (merge),
 * and on a row that contains nothing.
 */
export function nestingItems(subnet: SubnetResponse, nesting: SubnetNesting): CardFieldItem[] {
	if (isProvisionalCidr(subnet)) return [];

	const count = nesting.childrenOf(subnet.id).length;
	if (count === 0) return [];
	return [
		{
			id: 'contains',
			label: subnets_containsCount({ count }),
			color: 'Gray',
			title: subnets_containsDetail({ count })
		}
	];
}

export interface SubnetNesting {
	/** The subnets directly inside this one. */
	childrenOf: (id: string) => SubnetResponse[];
	/** The subnet's parent, when it is in the list. */
	parentOf: (subnet: SubnetResponse) => SubnetResponse | null;
	/** The widest range above this subnet, or the subnet itself when nothing contains it. */
	rootOf: (subnet: SubnetResponse) => SubnetResponse;
}

export function subnetNesting(subnets: SubnetResponse[]): SubnetNesting {
	const byId = new Map(subnets.map((s) => [s.id, s]));
	const children = new Map<string, SubnetResponse[]>();
	for (const subnet of subnets) {
		const parent = subnet.parent_subnet_id;
		if (!parent || !byId.has(parent)) continue;
		if (!children.has(parent)) children.set(parent, []);
		children.get(parent)!.push(subnet);
	}

	const parentOf = (subnet: SubnetResponse) =>
		(subnet.parent_subnet_id && byId.get(subnet.parent_subnet_id)) || null;

	return {
		childrenOf: (id) => children.get(id) ?? [],
		parentOf,
		rootOf: (subnet) => {
			let current = subnet;
			// Bounded by the list's size, so a malformed chain cannot loop.
			for (let step = 0; step < subnets.length; step++) {
				const parent = parentOf(current);
				if (!parent) break;
				current = parent;
			}
			return current;
		}
	};
}
