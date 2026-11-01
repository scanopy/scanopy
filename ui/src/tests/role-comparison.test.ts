import { describe, expect, it } from 'vitest';
import { roleColumns, roleLevel, roleSections } from '$lib/features/users/role-comparison';
import { permissions, productAreas, resources } from '$lib/shared/stores/metadata';

describe('role comparison', () => {
	it('lists every resource once, under its area, in area order', () => {
		const sections = roleSections();
		const listed = sections.flatMap((section) => section.rows);
		expect([...listed].sort()).toEqual(
			resources
				.getItems()
				.map((r) => r.id)
				.sort()
		);

		const order = productAreas.getItems().map((area) => area.id);
		const ranks = sections.map((section) => order.indexOf(section.id));
		expect(ranks).toEqual([...ranks].sort((a, b) => a - b));
		for (const section of sections) {
			for (const resource of section.rows) {
				expect(resources.getCategory(resource)).toBe(section.id);
			}
		}
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
			const id = resource.id as Parameters<typeof roleLevel>[0];
			expect(roleLevel(id, 'Owner')).toBe(roleLevel(id, 'Admin'));
		}
	});

	it('lists everything a role holds, each resource once, at the level the grid shows', () => {
		for (const role of permissions.getItems()) {
			const { access } = permissions.getMetadata(role.id);
			const listed = access.flatMap(({ level, resources: held }) =>
				held.map((resource) => {
					expect(roleLevel(resource, role.id)).toBe(level);
					return resource;
				})
			);
			const reached = resources
				.getItems()
				.map((resource) => resource.id as Parameters<typeof roleLevel>[0])
				.filter((resource) => roleLevel(resource, role.id) !== null);
			expect([...listed].sort()).toEqual([...reached].sort());
		}
	});
});
