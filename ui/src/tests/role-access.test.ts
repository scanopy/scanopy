import { describe, expect, it } from 'vitest';
import { actionColumns, roleActions, roleResources } from '$lib/features/users/role-access';
import { permissions, productAreas, resources } from '$lib/shared/stores/metadata';

type Resource = Parameters<typeof roleActions>[0];

describe('role access', () => {
	it('gives a role one row for each resource it can act on, in area order', () => {
		const order = productAreas.getItems().map((area) => area.id);
		for (const role of permissions.getItems()) {
			const rows = roleResources(role.id);
			expect(new Set(rows).size).toBe(rows.length);
			for (const resource of resources.getItems()) {
				const id = resource.id as Resource;
				expect(rows.includes(id)).toBe(roleActions(id, role.id).length > 0);
			}
			const ranks = rows.map((id) => order.indexOf(resources.getCategory(id) as string));
			expect(ranks).toEqual([...ranks].sort((a, b) => a - b));
		}
	});

	it('shows Create on Sites rather than a row for new sites', () => {
		for (const role of permissions.getItems()) {
			expect(roleResources(role.id)).not.toContain('NewSite');
		}
		expect(roleActions('Site', 'Admin')).toContain('Create');
		expect(roleActions('Site', 'Member')).not.toContain('Create');
	});

	it('reads a site-keyed row for Owner from the site role it maps to, which Admin holds too', () => {
		const siteKeyed = resources.getItems().filter((r) => r.metadata.per_site);
		expect(siteKeyed.length).toBeGreaterThan(0);
		for (const resource of siteKeyed) {
			const id = resource.id as Resource;
			expect(roleActions(id, 'Owner')).toEqual(roleActions(id, 'Admin'));
		}
	});

	it('has an icon for every action a role can hold', () => {
		const known = actionColumns().map((action) => action.id);
		for (const role of permissions.getItems()) {
			for (const resource of roleResources(role.id)) {
				expect(roleActions(resource, role.id).every((action) => known.includes(action))).toBe(true);
			}
		}
	});
});
