import { describe, it, expect } from 'vitest';
import { buildTreeSections, groupItems } from '$lib/shared/components/data/controls/grouping';
import type { FieldConfig } from '$lib/shared/components/data/types';
import {
	missingRootIds,
	virtualizationGroupKey,
	virtualizationGroupLabel,
	virtualizationTree
} from '$lib/features/hosts/virtualization-tree';

interface Row {
	id: string;
	display_name: string;
	virtualization_parent_host_id: string | null;
	virtualization_root_host_id: string | null;
	virtualization_depth: number;
	runtime: string | null;
}

const row = (
	id: string,
	parent: string | null,
	root: string | null,
	depth: number,
	runtime: string | null = null
): Row => ({
	id,
	display_name: id,
	virtualization_parent_host_id: parent,
	virtualization_root_host_id: root,
	virtualization_depth: depth,
	runtime
});

const labels = { notVirtualized: 'Not virtualized', unknownRoot: 'Unknown host' };
const NUL = String.fromCharCode(0);

// A page holding a VM and its container, but not the Proxmox node above them.
const vm = row('docker-vm', 'pve-01', 'pve-01', 1, 'Proxmox VE');
const container = row('pihole', 'docker-vm', 'pve-01', 2, 'Docker');
const lone = row('printer', null, null, 0);
const page = [lone, vm, container];

describe('virtualization tree grouping', () => {
	it('asks for the names of roots that sit on another page, once each', () => {
		expect(missingRootIds(page)).toEqual(['pve-01']);
		expect(missingRootIds([row('pve-01', null, 'pve-01', 0), vm])).toEqual([]);
	});

	it('keys every host by its root as the server counts it, and a host in no tree by the empty string', () => {
		expect(page.map(virtualizationGroupKey)).toEqual(['', 'pve-01', 'pve-01']);
	});

	it('heads a group with its root, even when only the runtime one level up is on the page', () => {
		const roots = new Map([['pve-01', { display_name: 'pve-01' }]]);
		expect(virtualizationGroupLabel(container, roots, labels)).toBe('pve-01');
		expect(virtualizationGroupLabel(lone, roots, labels)).toBe('Not virtualized');
		expect(virtualizationGroupLabel(container, new Map(), labels)).toBe('Unknown host');
	});

	it('groups by the tree while the column keeps showing the runtime', () => {
		const roots = new Map([['pve-01', { display_name: 'pve-01' }]]);
		const fields: FieldConfig<Row>[] = [
			{
				orderField: 'virtualized_by',
				label: 'Virtualized By',
				type: 'string',
				getValue: (r) => r.runtime,
				getGroupValue: virtualizationGroupKey,
				getGroupLabel: (r) => virtualizationGroupLabel(r, roots, labels),
				tree: virtualizationTree
			}
		];

		const groups = groupItems(
			page,
			fields,
			'virtualized_by',
			{ ungrouped: '-', yes: 'Yes', no: 'No' },
			true
		);
		expect([...groups.entries()].map(([name, rows]) => [name, rows.map((r) => r.id)])).toEqual([
			['Not virtualized', ['printer']],
			['pve-01', ['docker-vm', 'pihole']]
		]);
	});

	it('heads a section with the guest that runs others, even when its root is on another page', () => {
		const entries = buildTreeSections([vm, container], virtualizationTree, true, 'pve-01');
		expect(entries).toEqual([
			{
				type: 'section',
				key: `pve-01${NUL}docker-vm`,
				item: vm,
				count: 1,
				depth: 0,
				entries: [{ type: 'row', item: container, depth: 1 }]
			}
		]);
	});
});
