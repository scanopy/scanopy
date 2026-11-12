import { describe, expect, it } from 'vitest';
import serviceDefinitionsJson from '#lib/data/service-definitions.json';
import { guestCountLabel } from '#lib/features/services/virtualized-count.ts';

/** Every definition that runs guests, by the kind of guest it runs, from the backend's fixture. */
const VIRTUALIZERS = serviceDefinitionsJson
	.map((d) => ({
		id: d.id,
		kind: (d.metadata as { manages_virtualization?: string | null }).manages_virtualization
	}))
	.filter((d): d is { id: string; kind: string } => Boolean(d.kind));

describe('guest count labels', () => {
	it('has a runtime, a hypervisor and an identity holder to compare', () => {
		const kinds = new Set(VIRTUALIZERS.map((d) => d.kind));
		expect(kinds).toEqual(new Set(['containers', 'vms', 'identities']));
	});

	it('names the count and one noun per kind of guest', () => {
		const labelsByKind = new Map<string, Set<string>>();
		for (const { id, kind } of VIRTUALIZERS) {
			const label = guestCountLabel(id, 7);
			expect(label, id).toContain('7');
			labelsByKind.set(kind, (labelsByKind.get(kind) ?? new Set()).add(label));
		}
		// Every definition of a kind reads the same, and no two kinds read alike: Docker and
		// Podman never say VMs, and Proxmox VE never says containers.
		for (const labels of labelsByKind.values()) expect(labels.size).toBe(1);
		const distinct = new Set([...labelsByKind.values()].map((labels) => [...labels][0]));
		expect(distinct.size).toBe(labelsByKind.size);
	});
});
