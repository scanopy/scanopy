import { describe, it, expect } from 'vitest';
import {
	buildTreeSections,
	flattenTreeEntries,
	isSingleRootTree,
	treeDepthViolations,
	type TreeEntry
} from '$lib/shared/components/data/controls/grouping';
import { compareByField } from '$lib/shared/components/data/controls/sorting';
import type { FieldConfig, TreeConfig } from '$lib/shared/components/data/types';
import { compareCidr } from '$lib/shared/utils/cidr';
import { nestedRangeOf, subnetNesting } from '$lib/features/subnets/nesting';
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

/** A group's entries as `depth|id` for a leaf row and an object for a row with children. */
function shape(entries: TreeEntry<Range>[]): unknown[] {
	return entries.map((entry) =>
		entry.type === 'row'
			? `${entry.depth}|${entry.item.id}`
			: {
					section: `${entry.depth}|${entry.item.id}`,
					count: entry.count,
					depth: entry.depth,
					entries: shape(entry.entries)
				}
	);
}

describe('buildTreeSections', () => {
	it('nests a full list three levels deep, each parent its own row with its children under it', () => {
		const entries = buildTreeSections([leaf, sibling, wide, mid], tree, false, 'g', byCidr);
		// No parent appears inside its own section: wide is the row, its children follow one in.
		expect(shape(entries)).toEqual([
			{
				section: '0|wide',
				count: 3,
				depth: 0,
				entries: ['1|sibling', { section: '1|mid', count: 1, depth: 1, entries: ['2|leaf'] }]
			}
		]);
		expect(flattenTreeEntries(entries).map((r) => r.id)).toEqual([
			'wide',
			'sibling',
			'mid',
			'leaf'
		]);
	});

	it('gives a row whose parent is filtered out a section of its own at the top', () => {
		const entries = buildTreeSections([leaf, sibling, mid], tree, false, 'g', byCidr);
		expect(shape(entries)).toEqual([
			'0|sibling',
			{ section: '0|mid', count: 1, depth: 0, entries: ['1|leaf'] }
		]);
	});

	it('keeps arrival order among siblings without a comparator', () => {
		const entries = buildTreeSections([wide, mid, sibling], tree, false, 'g');
		expect(flattenTreeEntries(entries).map((r) => r.id)).toEqual(['wide', 'mid', 'sibling']);
	});

	it('survives a cycle instead of recursing forever', () => {
		const a: Range = { id: 'a', cidr: '10.0.0.0/24', parent: 'b' };
		const b: Range = { id: 'b', cidr: '10.0.1.0/24', parent: 'a' };
		const root: Range = { id: 'root', cidr: '10.0.2.0/24', parent: null };
		const entries = buildTreeSections([a, b, root], tree, false, 'g');
		// Neither cycle member has a parent outside the cycle, so only the real root is reached.
		expect(shape(entries)).toEqual(['0|root']);
	});

	it('nests a server page from depth, rows below an earlier page starting at the top', () => {
		const depthTree = { ...tree, depth: (r: Range) => r.depth ?? 0 };
		const other: Range = { id: 'other', cidr: '10.10.32.0/24', parent: 'wide', depth: 1 };
		const page = [{ ...leaf, depth: 2 }, other];
		expect(shape(buildTreeSections(page, depthTree, true, 'g'))).toEqual(['0|leaf', '0|other']);

		const full = [{ ...wide, depth: 0 }, { ...mid, depth: 1 }, { ...leaf, depth: 2 }, other];
		expect(shape(buildTreeSections(full, depthTree, true, 'g'))).toEqual([
			{
				section: '0|wide',
				count: 3,
				depth: 0,
				entries: [{ section: '1|mid', count: 1, depth: 1, entries: ['2|leaf'] }, '1|other']
			}
		]);
	});

	it('keys sections by path, so same-labelled rows in different places stay apart', () => {
		const twinA: Range = { id: 'twin-a', cidr: '10.0.0.0/24', parent: null };
		const twinB: Range = { id: 'twin-b', cidr: '10.0.0.0/24', parent: null };
		const kidA: Range = { id: 'kid-a', cidr: '10.0.0.0/25', parent: 'twin-a' };
		const kidB: Range = { id: 'kid-b', cidr: '10.0.0.0/25', parent: 'twin-b' };
		const entries = buildTreeSections([twinA, kidA, twinB, kidB], tree, false, 'g');
		const keys = entries.map((entry) => (entry.type === 'section' ? entry.key : null));
		expect(keys[0]).not.toBeNull();
		expect(new Set(keys).size).toBe(2);
		expect(
			buildTreeSections([twinA, kidA, twinB, kidB], tree, false, 'other').map((entry) =>
				entry.type === 'section' ? entry.key : null
			)
		).not.toEqual(keys);
	});
});

describe('isSingleRootTree', () => {
	it('lets a group that is one tree under a real root drop its header', () => {
		const entries = buildTreeSections([wide, mid, leaf, sibling], tree, false, 'g', byCidr);
		expect(isSingleRootTree(entries, tree)).toBe(true);
	});

	it('keeps the header when the top row has a parent that is filtered out or on another page', () => {
		const entries = buildTreeSections([mid, leaf], tree, false, 'g', byCidr);
		expect(isSingleRootTree(entries, tree)).toBe(false);
	});

	it('keeps the header for a group of several top-level rows', () => {
		const other: Range = { id: 'other', cidr: '192.168.0.0/24', parent: null };
		const entries = buildTreeSections([wide, mid, other], tree, false, 'g', byCidr);
		expect(isSingleRootTree(entries, tree)).toBe(false);
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

	it('groups a nested subnet under its widest range, and one that nests with nothing under none', () => {
		expect(nestedRangeOf(find('leaf'), nesting)).toBe('10.10.0.0/16');
		expect(nestedRangeOf(find('wide'), nesting)).toBe('10.10.0.0/16');
		expect(nestedRangeOf(find('lone'), nesting)).toBeNull();
		expect(nestedRangeOf(find('orphan'), nesting)).toBeNull();
	});
});
