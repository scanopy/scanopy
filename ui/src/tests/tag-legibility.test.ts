import { describe, it, expect } from 'vitest';
import {
	credentialTypes,
	dependencyTypes,
	discoveryProtocols,
	permissions,
	proxmoxGuestTypes,
	containerNetworkTypes,
	serviceCategories,
	serviceDefinitions,
	serviceVirtualizations,
	subnetTypes,
	ifOperStatuses
} from '$lib/shared/stores/metadata';
import type { TagProps } from '$lib/shared/components/data/types';
import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
import { ServiceTypeDisplay } from '$lib/shared/components/forms/selection/display/ServiceTypeDisplay.svelte';
import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
import type { Service } from '$lib/features/services/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';
import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
import type { Interface } from '$lib/features/hosts/types/base';
import { common_unknown, hosts_interfaces_operStatusNotReported } from '$lib/paraglide/messages';

// Every store a tag is built from: the tag reads the value's display name, and its tooltip is
// what the value means, with no "Category:"-style prefix naming the dimension.
const TAG_STORES = {
	serviceCategories,
	serviceVirtualizations,
	discoveryProtocols,
	subnetTypes,
	proxmoxGuestTypes,
	containerNetworkTypes,
	permissions,
	credentialTypes,
	dependencyTypes,
	ifOperStatuses
};

describe('getTag', () => {
	for (const [name, store] of Object.entries(TAG_STORES)) {
		it(`${name}: label is the display name, title is the description alone`, () => {
			const items = store.getItems();
			expect(items.length).toBeGreaterThan(0);
			for (const item of items) {
				const tag = store.getTag(item.id);
				expect(tag.label).toBe(store.getName(item.id));
				expect(tag.title).toBe(store.getDescription(item.id) || undefined);
			}
		});
	}
});

/** A tag explains itself: by tooltip, or by the entity popover an `entityRef` opens. */
function explained(tag: TagProps): boolean {
	return !!tag.title || !!tag.entityRef;
}

describe('display tags explain themselves', () => {
	it('every ServiceDisplay tag, for every definition, in and out of a container', () => {
		for (const definition of serviceDefinitions.getItems()) {
			for (const type of serviceVirtualizations.getItems().map((v) => v.id)) {
				const service = {
					id: 's',
					name: 'x',
					service_definition: definition.id,
					bindings: [],
					virtualization_metadata: { type, details: {} }
				} as unknown as Service;
				const tags = ServiceDisplay.getTags!(service, {});
				expect(tags.every(explained)).toBe(true);
				// The runtime tag reads its display name, not the raw `virtualization_metadata.type`.
				expect(tags.map((t) => t.label)).toContain(serviceVirtualizations.getName(type));
			}
		}
	});

	it('ServiceTypeDisplay shows the category name, never its raw id', () => {
		for (const definition of serviceDefinitions.getItems()) {
			const [tag] = ServiceTypeDisplay.getTags!(definition as never, {});
			expect(explained(tag)).toBe(true);
			expect(tag.label).toBe(serviceCategories.getName(definition.category));
		}
	});

	it('InterfaceDisplay names the link status with no "Status:" prefix', () => {
		for (const status of ifOperStatuses.getItems()) {
			const iface = { id: 'i', if_name: 'eth0', oper_status: status.id } as unknown as Interface;
			const [tag] = InterfaceDisplay.getTags!(iface, undefined);
			expect(tag.label).toBe(ifOperStatuses.getName(status.id));
			expect(explained(tag)).toBe(true);
		}
		// Unreported (a PROFINET DCP identify carries a MAC only): Unknown, and the tooltip says why.
		const unreported = { id: 'i', if_name: 'eth0', oper_status: null } as unknown as Interface;
		const [tag] = InterfaceDisplay.getTags!(unreported, undefined);
		expect(tag.label).toBe(common_unknown());
		expect(tag.title).toBe(hosts_interfaces_operStatusNotReported());
	});

	it('SubnetDisplay shows the subnet type name, never its raw id', () => {
		for (const type of subnetTypes.getItems()) {
			const subnet = {
				id: 'n',
				name: 'lan',
				cidr: '10.0.0.0/24',
				subnet_type: type.id
			} as unknown as Subnet;
			for (const tag of SubnetDisplay.getTags!(subnet, {})) {
				expect(explained(tag)).toBe(true);
				expect(tag.label).toBe(subnetTypes.getName(type.id));
			}
		}
	});
});
