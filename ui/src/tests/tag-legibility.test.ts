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
	tagTitle
} from '$lib/shared/stores/metadata';
import type { TagProps } from '$lib/shared/components/data/types';
import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
import { ServiceTypeDisplay } from '$lib/shared/components/forms/selection/display/ServiceTypeDisplay.svelte';
import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
import type { Service } from '$lib/features/services/types/base';
import type { Subnet } from '$lib/features/subnets/types/base';

describe('tagTitle', () => {
	it('names the dimension and carries the description', () => {
		const title = tagTitle('Category', 'Container orchestration platforms');
		expect(title).toContain('Category');
		expect(title).toContain('Container orchestration platforms');
	});

	it('falls back to the dimension alone when there is no description', () => {
		expect(tagTitle('Category', '')).toBe('Category');
		expect(tagTitle('Category', null)).toBe('Category');
	});
});

// Every store a tag is built from: the tag reads the value's display name and its tooltip says
// what the tag is and what the value means.
const TAG_STORES = {
	serviceCategories,
	serviceVirtualizations,
	discoveryProtocols,
	subnetTypes,
	proxmoxGuestTypes,
	containerNetworkTypes,
	permissions,
	credentialTypes,
	dependencyTypes
};

describe('getTag', () => {
	for (const [name, store] of Object.entries(TAG_STORES)) {
		it(`${name}: label is the display name, title names the dimension and the meaning`, () => {
			const items = store.getItems();
			expect(items.length).toBeGreaterThan(0);
			for (const item of items) {
				const tag = store.getTag(item.id, 'Dimension');
				expect(tag.label).toBe(store.getName(item.id));
				expect(tag.title).toContain('Dimension');
				const description = store.getDescription(item.id);
				if (description) expect(tag.title).toContain(description);
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
