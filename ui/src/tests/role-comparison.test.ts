import { describe, expect, it } from 'vitest';
import {
	actionColumns,
	roleActions,
	roleColumns,
	roleResources,
	roleSections
} from '$lib/features/users/role-comparison';
import { permissions, productAreas, resources } from '$lib/shared/stores/metadata';

type Resource = Parameters<typeof roleActions>[0];

describe('role comparison', () => {
	it('lists each resource at most once, under its area, in area order', () => {
		const sections = roleSections();
		const listed = sections.flatMap((section) => section.rows);
		expect(new Set(listed).size).toBe(listed.length);

		const order = productAreas.getItems().map((area) => area.id);
		const ranks = sections.map((section) => order.indexOf(section.id));
		expect(ranks).toEqual([...ranks].sort((a, b) => a - b));
		for (const section of sections) {
			for (const resource of section.rows) {
				expect(resources.getCategory(resource)).toBe(section.id);
			}
		}
	});

	it('leaves out only resources no role shows, and shows Create on sites instead', () => {
		const listed = roleSections().flatMap((section) => section.rows);
		for (const resource of resources.getItems()) {
			const id = resource.id as Resource;
			const shown = roleColumns().some((role) => roleActions(id, role.id).length > 0);
			expect(listed.includes(id)).toBe(shown);
		}
		expect(roleActions('Site' as Resource, 'Admin')).toContain('Create');
		expect(listed).not.toContain('NewSite');
	});

	it('shows the four roles, lowest first', () => {
		const columns = roleColumns().map((column) => column.id);
		expect(columns).toHaveLength(permissions.getItems().length);
		for (const [index, role] of columns.entries()) {
			const grantable = permissions.getMetadata(role).grantable_api_key_permissions;
			expect([...grantable].sort()).toEqual(columns.slice(0, index + 1).sort());
		}
	});

	it('reads a site-keyed row for Owner from the site role it maps to, which Admin holds too', () => {
		const siteKeyed = resources.getItems().filter((r) => r.metadata.per_site);
		expect(siteKeyed.length).toBeGreaterThan(0);
		for (const resource of siteKeyed) {
			const id = resource.id as Resource;
			expect(roleActions(id, 'Owner')).toEqual(roleActions(id, 'Admin'));
		}
	});

	it('gives a role a row for exactly the resources it can act on', () => {
		const known = actionColumns().map((action) => action.id);
		for (const role of permissions.getItems()) {
			for (const section of roleSections()) {
				for (const resource of section.rows) {
					const held = roleActions(resource, role.id);
					expect(roleResources(role.id).includes(resource)).toBe(held.length > 0);
					expect(held.every((action) => known.includes(action))).toBe(true);
				}
			}
		}
	});
});
