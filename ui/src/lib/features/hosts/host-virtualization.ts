/**
 * One sentence saying what a virtualized host is and what runs it: "A virtual machine managed by
 * Proxmox, running on pve-01."
 */

import type { HostVirtualization } from '$lib/features/hosts/types/base';
import { hostVirtualizations } from '$lib/shared/stores/metadata';
import {
	hosts_virtualizationSummary_container,
	hosts_virtualizationSummary_lxc,
	hosts_virtualizationSummary_networkIdentity,
	hosts_virtualizationSummary_networkIdentityNoInterface,
	hosts_virtualizationSummary_unknownOwner,
	hosts_virtualizationSummary_vm
} from '$lib/paraglide/messages';

/**
 * `ownerName` is the display name of the host doing the virtualizing, `null` until it is known.
 * `interfaceName` names the interface presenting a network identity, resolved from
 * `virtualization_interface_id`; `null` when the host has none or it isn't loaded.
 */
export function virtualizationSummary(
	virtualization: HostVirtualization,
	ownerName: string | null,
	interfaceName: string | null = null
): string {
	const host = ownerName ?? hosts_virtualizationSummary_unknownOwner();
	const manager = hostVirtualizations.getName(virtualization.type);
	switch (virtualization.type) {
		case 'Proxmox':
			return virtualization.details.guest_type === 'Lxc'
				? hosts_virtualizationSummary_lxc({ manager, host })
				: hosts_virtualizationSummary_vm({ manager, host });
		case 'VCenter':
		case 'ESXi':
			return hosts_virtualizationSummary_vm({ manager, host });
		case 'Docker':
		case 'Podman':
			return hosts_virtualizationSummary_container({ manager, host });
		case 'NetworkIdentity': {
			const iface = interfaceName?.trim();
			return iface
				? hosts_virtualizationSummary_networkIdentity({ host, interface: iface })
				: hosts_virtualizationSummary_networkIdentityNoInterface({ host });
		}
	}
}
