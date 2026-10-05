import { describe, it, expect } from 'vitest';
import { arrangeTree, treeDepthViolations } from '$lib/shared/components/data/controls/grouping';
import { compareByField } from '$lib/shared/components/data/controls/sorting';
import type { FieldConfig, TreeConfig } from '$lib/shared/components/data/types';
import { compareCidr } from '$lib/shared/utils/cidr';
import { subnetNesting } from '$lib/features/subnets/nesting';
import type { SubnetResponse } from '$lib/features/subnets/types/base';

interface Range {
	id: string;
	cidr: string;
	parent: string | null;
	depth?: number;
}

const tree: TreeConfig<Range> = {
	key: (r) => r.id,
	parentKey: (r) => r.parent
};

const byCidr = (a: Range, b: Range) => compareCidr(a.cidr, b.cidr);

const wide: Range = { id: 'wide', cidr: '10.10.0.0/16', parent: null };
const mid: Range = { id: 'mid', cidr: '10.10.16.0/20', parent: 'wide' };
const leaf: Range = { id: 'leaf', cidr: '10.10.16.0/24', parent: 'mid' };
const sibling: Range = { id: 'sibling', cidr: '10.10.2.0/24', parent: 'wide' };

describe('compareCidr', () => {
	it('orders by address rather than text, so each range precedes the ranges inside it', () => {
		const sorted = [leaf, sibling, mid, wide].map((r) => r.cidr).sort(compareCidr);
		expect(sorted).toEqual(['10.10.0.0/16', '10.10.2.0/24', '10.10.16.0/20', '10.10.16.0/24']);
	});

	it('puts IPv4 before IPv6 and unparseable values last', () => {
		expect(['not-a-cidr', 'fd00::/64', '192.168.1.0/24'].sort(compareCidr)).toEqual([
			'192.168.1.0/24',
			'fd00::/64',
			'not-a-cidr'
		]);
	});

	it('drives a field sort through `compare`, reversed for descending', () => {
		const field: FieldConfig<Range> = {
			key: 'cidr',
			label: 'CIDR',
			type: 'string',
			getValue: (r) => r.cidr,
			compare: byCidr
		};
		expect(compareByField(sibling, mid, field, 'asc')).toBeLessThan(0);
		expect(compareByField(sibling, mid, field, 'desc')).toBeGreaterThan(0);
	});
});

describe('arrangeTree', () => {
	it('orders a full list parent-first and counts depth from the roots', () => {
		const layout = arrangeTree([leaf, sibling, wide, mid], tree, false, byCidr);
		expect(layout.items.map((r) => r.id)).toEqual(['wide', 'sibling', 'mid', 'leaf']);
		expect(Object.fromEntries(layout.depths)).toEqual({ wide: 0, sibling: 1, mid: 1, leaf: 2 });
	});

	it('makes a row whose parent is filtered out a root', () => {
		const layout = arrangeTree([leaf, sibling, mid], tree, false, byCidr);
		expect(layout.items.map((r) => r.id)).toEqual(['sibling', 'mid', 'leaf']);
		expect(Object.fromEntries(layout.depths)).toEqual({ sibling: 0, mid: 0, leaf: 1 });
	});

	it('keeps arrival order among siblings without a comparator', () => {
		const layout = arrangeTree([wide, mid, sibling], tree, false);
		expect(layout.items.map((r) => r.id)).toEqual(['wide', 'mid', 'sibling']);
	});

	it('survives a cycle instead of recursing forever', () => {
		const a: Range = { id: 'a', cidr: '10.0.0.0/24', parent: 'b' };
		const b: Range = { id: 'b', cidr: '10.0.1.0/24', parent: 'a' };
		const root: Range = { id: 'root', cidr: '10.0.2.0/24', parent: null };
		const layout = arrangeTree([a, b, root], tree, false);
		// Neither cycle member has a parent outside the cycle, so only the real root is reached.
		expect(layout.items.map((r) => r.id)).toEqual(['root']);
	});

	it('trusts the server on a paginated list: arrival order, server depth', () => {
		const paged = [
			{ ...mid, depth: 1 },
			{ ...leaf, depth: 2 }
		];
		const layout = arrangeTree(paged, { ...tree, depth: (r) => r.depth ?? 0 }, true, byCidr);
		expect(layout.items.map((r) => r.id)).toEqual(['mid', 'leaf']);
		expect(Object.fromEntries(layout.depths)).toEqual({ mid: 1, leaf: 2 });
	});
});

describe('treeDepthViolations', () => {
	const field: FieldConfig<Range> = {
		key: 'range',
		label: 'Range',
		type: 'string',
		groupable: true,
		tree
	};

	it('flags a tree without server depth on a paginated list only', () => {
		expect(treeDepthViolations([field], true)).toEqual(['range']);
		expect(treeDepthViolations([field], false)).toEqual([]);
		expect(treeDepthViolations([{ ...field, tree: { ...tree, depth: () => 0 } }], true)).toEqual(
			[]
		);
	});
});

describe('subnetNesting', () => {
	function subnet(id: string, cidr: string, parent: string | null): SubnetResponse {
		return {
			id,
			cidr,
			name: id,
			parent_subnet_id: parent,
			used_addresses: 0,
			usable_addresses: 0
		} as SubnetResponse;
	}

	const subnets = [
		subnet('wide', '10.10.0.0/16', null),
		subnet('mid', '10.10.16.0/20', 'wide'),
		subnet('leaf', '10.10.16.0/24', 'mid'),
		subnet('lone', '192.168.1.0/24', null),
		subnet('orphan', '172.16.1.0/24', 'not-listed')
	];
	const nesting = subnetNesting(subnets);
	const find = (id: string) => subnets.find((s) => s.id === id)!;

	it('walks to the widest range above a subnet', () => {
		expect(nesting.rootOf(find('leaf')).id).toBe('wide');
		expect(nesting.rootOf(find('wide')).id).toBe('wide');
		expect(nesting.rootOf(find('lone')).id).toBe('lone');
	});

	it('derives children from the parent ids', () => {
		expect(nesting.childrenOf('wide').map((s) => s.id)).toEqual(['mid']);
		expect(nesting.childrenOf('leaf')).toEqual([]);
	});

	it('treats a parent missing from the list as no parent', () => {
		expect(nesting.parentOf(find('orphan'))).toBeNull();
		expect(nesting.rootOf(find('orphan')).id).toBe('orphan');
	});
});
