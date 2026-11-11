import { describe, it, expect } from 'vitest';
import {
	buildTreeSections,
	flattenTreeEntries,
	mergeRootedTrees,
	rootedTreesPosition,
	ROOTED_TREES_KEY,
	type RenderGroup,
	type TreeEntry
} from '#lib/shared/components/data/controls/grouping.ts';
import type { TreeConfig } from '#lib/shared/components/data/types.ts';
import { subnetNesting } from '#lib/features/subnets/nesting.ts';
import type { SubnetResponse } from '#lib/features/subnets/types/base.ts';

interface Range {
	id: string;
	parent: string | null;
	depth: number;
	root: string | null;
}

const tree: TreeConfig<Range> = {
	key: (r) => r.id,
	parentKey: (r) => r.parent,
	depth: (r) => r.depth,
	rootKey: (r) => r.root,
	rootsLabel: () => 'Nested'
};

// In the server's order: each range directly before the ranges inside it.
const wide: Range = { id: 'wide', parent: null, depth: 0, root: 'wide' };
const sibling: Range = { id: 'sibling', parent: 'wide', depth: 1, root: 'wide' };
const mid: Range = { id: 'mid', parent: 'wide', depth: 1, root: 'wide' };
const leaf: Range = { id: 'leaf', parent: 'mid', depth: 2, root: 'wide' };

/** A group's entries as `depth|id` for a leaf row and an object for a row with children. */
function shape(entries: TreeEntry<Range>[]): unknown[] {
	return entries.map((entry) =>
		entry.type === 'continued'
			? { continued: entry.label, entries: shape(entry.entries) }
			: entry.type === 'row'
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
	it('nests three levels deep, each parent its own row with its children under it', () => {
		const entries = buildTreeSections([wide, sibling, mid, leaf], tree, 'g');
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

	it('starts rows whose ancestors are on an earlier page at the top', () => {
		const other: Range = { id: 'other', parent: 'wide', depth: 1, root: 'wide' };
		expect(shape(buildTreeSections([leaf, other], tree, 'g'))).toEqual(['0|leaf', '0|other']);
		expect(shape(buildTreeSections([sibling, mid, leaf], tree, 'g'))).toEqual([
			'0|sibling',
			{ section: '0|mid', count: 1, depth: 0, entries: ['1|leaf'] }
		]);
	});

	it('keys sections by path, so same-labelled rows in different places stay apart', () => {
		const twinA: Range = { id: 'twin-a', parent: null, depth: 0, root: 'twin-a' };
		const twinB: Range = { id: 'twin-b', parent: null, depth: 0, root: 'twin-b' };
		const kidA: Range = { id: 'kid-a', parent: 'twin-a', depth: 1, root: 'twin-a' };
		const kidB: Range = { id: 'kid-b', parent: 'twin-b', depth: 1, root: 'twin-b' };
		const entries = buildTreeSections([twinA, kidA, twinB, kidB], tree, 'g');
		const keys = entries.map((entry) => (entry.type === 'section' ? entry.key : null));
		expect(keys[0]).not.toBeNull();
		expect(new Set(keys).size).toBe(2);
		expect(
			buildTreeSections([twinA, kidA, twinB, kidB], tree, 'other').map((entry) =>
				entry.type === 'section' ? entry.key : null
			)
		).not.toEqual(keys);
	});
});

describe('mergeRootedTrees', () => {
	const group = (key: string, rows: Range[], range: RenderGroup<Range>['range'] = null) => {
		const entries = buildTreeSections(rows, tree, key);
		return { key, name: key, items: rows, range, entries, rootRanges: null };
	};
	const other: Range = { id: 'other', parent: null, depth: 0, root: 'other' };
	const otherKid: Range = { id: 'other-kid', parent: 'other', depth: 1, root: 'other' };
	const loose: Range = { id: 'loose', parent: null, depth: 0, root: null };

	it('collects every rooted tree under one header, where the first one was', () => {
		const range = { start: 1, end: 3, total: 3 };
		const groups = [
			group('Ungrouped', [loose]),
			group('wide', [wide, mid, leaf], range),
			group('other', [other, otherKid])
		];
		const merged = mergeRootedTrees(groups, tree, null);
		expect(merged.map((g) => g.name)).toEqual(['Ungrouped', 'Nested']);
		const [, trees] = merged;
		expect(trees.key).toBe(ROOTED_TREES_KEY);
		// Roots are ordinary rows of the merged group, not headers of their own.
		expect(
			trees.entries!.map((entry) => (entry.type === 'continued' ? null : entry.item.id))
		).toEqual(['wide', 'other']);
		expect(trees.items.map((r) => r.id)).toEqual(['wide', 'mid', 'leaf', 'other', 'other-kid']);
		expect(trees.rootRanges!.get('wide')).toEqual(range);
		expect(trees.rootRanges!.has('other')).toBe(false);
	});

	it('nests a tree whose root is on an earlier page under a head naming the root', () => {
		// Page 2 of the "wide" tree: the root and `sibling` are on page 1.
		const range = { start: 6, end: 25, total: 30 };
		const groups = [group('10.10.0.0/16', [mid, leaf], range), group('Ungrouped', [loose])];
		const merged = mergeRootedTrees(groups, tree, null);
		expect(merged.map((g) => g.name)).toEqual(['Nested', 'Ungrouped']);
		const [head] = merged[0].entries!;
		expect(head).toMatchObject({ type: 'continued', label: '10.10.0.0/16', range });
		// One level in from the head, as they would sit under the root.
		expect(shape(head.type === 'continued' ? head.entries : [])).toEqual([
			{ section: '1|mid', count: 1, depth: 1, entries: ['2|leaf'] }
		]);
		// The head is not a row of the page.
		expect(flattenTreeEntries(merged[0].entries!).map((r) => r.id)).toEqual(['mid', 'leaf']);
	});

	it('leaves groups alone when none is a tree', () => {
		const groups = [group('Ungrouped', [loose])];
		expect(mergeRootedTrees(groups, tree, null)).toBe(groups);
	});

	it('heads the merged group with the range every tree together spans', () => {
		const range = { start: 4, end: 20, total: 159 };
		const merged = mergeRootedTrees([group('wide', [wide, mid, leaf])], tree, range);
		expect(merged[0].range).toEqual(range);
	});
});

describe('rootedTreesPosition', () => {
	it('spans every tree group, which the server lists after the rows in no tree', () => {
		const counts = [
			{ value: '', count: 57 },
			{ value: 'a', count: 25 },
			{ value: 'b', count: 9 },
			{ value: 'c', count: 25 }
		];
		expect(rootedTreesPosition(counts)).toEqual({ start: 57, count: 59 });
	});

	it('is null with no tree groups or no counts', () => {
		expect(rootedTreesPosition([{ value: '', count: 3 }])).toBeNull();
		expect(rootedTreesPosition(null)).toBeNull();
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
