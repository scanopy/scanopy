import { describe, expect, it } from 'vitest';
import { virtualizationSummary } from '$lib/features/hosts/host-virtualization';
import type { HostVirtualization } from '$lib/features/hosts/types/base';
import { hosts_virtualizationSummary_unknownOwner } from '$lib/paraglide/messages';

/** One of every virtualization type, so a new variant without a sentence fails here. */
const everyType: HostVirtualization[] = [
	{ type: 'Proxmox', details: { guest_type: 'Qemu', vm_id: '101', vm_name: 'web' } },
	{ type: 'Proxmox', details: { guest_type: 'Lxc', vm_id: '102', vm_name: 'dns' } },
	{ type: 'VCenter', details: { vm_id: 'vm-12', vm_name: 'db' } },
	{ type: 'ESXi', details: { vm_id: '7', vm_name: 'mail' } },
	{ type: 'Docker', details: { container_name: 'pihole', network_type: 'MacVlan' } },
	{ type: 'Podman', details: { container_name: 'unifi', network_type: 'IpVlan' } },
	{ type: 'NetworkIdentity', details: {} }
];

describe('virtualizationSummary', () => {
	it('names the owner host for every virtualization type', () => {
		for (const virtualization of everyType) {
			expect(virtualizationSummary(virtualization, 'pve-01')).toContain('pve-01');
		}
	});

	it('stands in for an owner that is not known yet', () => {
		for (const virtualization of everyType) {
			expect(virtualizationSummary(virtualization, null)).toContain(
				hosts_virtualizationSummary_unknownOwner()
			);
		}
	});

	it('tells a Proxmox LXC container apart from a Proxmox VM', () => {
		const [vm, lxc] = everyType;
		expect(virtualizationSummary(vm, 'pve-01')).not.toEqual(virtualizationSummary(lxc, 'pve-01'));
	});

	it('names the interface of a site identity only when it has one', () => {
		const identity: HostVirtualization = { type: 'NetworkIdentity', details: {} };
		expect(virtualizationSummary(identity, 'switch-01', 'mv-snmp4')).toContain('mv-snmp4');
		expect(virtualizationSummary(identity, 'switch-01', null)).not.toMatch(/null|undefined/);
	});
});
