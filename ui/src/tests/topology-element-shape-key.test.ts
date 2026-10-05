import { describe, it, expect } from 'vitest';
import {
	elementShapeKey,
	type ElementRenderResult
} from '$lib/features/topology/element-render-data';

/**
 * The shape key decides which element cards the measure pass can treat as
 * interchangeable. Under-discriminating is the dangerous direction: two cards
 * that render at different heights sharing a key means one of them is laid out
 * with the wrong height, silently.
 *
 * These assert the discrimination behaviour, not the key's literal format —
 * the format is free to change as long as it keeps separating these cases.
 */

const FLAGS = {
	inlineEntities: ['Service', 'Port'],
	inlinesService: true,
	inlinesPort: true,
	serviceInlineHidden: false,
	portInlineHidden: false
};

function service(id: string, name: string, portBindings = 0) {
	return {
		id,
		name,
		bindings: Array.from({ length: portBindings }, () => ({ type: 'Port' }))
	};
}

function result(overrides: Record<string, unknown> = {}, extra: Partial<ElementRenderResult> = {}) {
	return {
		flags: FLAGS,
		staleTag: null,
		data: {
			elementType: 'Host',
			headerText: 'web-01',
			subtitleText: null,
			bodyText: null,
			footerText: null,
			showServices: true,
			services: [],
			hiddenOpenPorts: [],
			ip_address_id: '',
			inlineGroups: [],
			...overrides
		},
		...extra
	} as unknown as ElementRenderResult;
}

describe('elementShapeKey', () => {
	it('matches cards that render identically but describe different entities', () => {
		// The whole point: two hosts with the same card structure must share a key
		// even though their names and ids differ.
		expect(elementShapeKey(result({ headerText: 'web-01' }))).toBe(
			elementShapeKey(result({ headerText: 'web-02' }))
		);
	});

	it('separates cards with different service counts', () => {
		expect(elementShapeKey(result({ services: [service('a', 'nginx')] }))).not.toBe(
			elementShapeKey(result({ services: [service('a', 'nginx'), service('b', 'redis')] }))
		);
	});

	it('separates cards whose inline group holds a different number of member hosts', () => {
		const member = (id: string) => ({
			host: { id, display_name: id },
			services: [service(`${id}-snmp`, 'SNMP')]
		});
		const group = (hosts: string[]) => ({
			groupId: 'identities',
			collapsed: false,
			header: service('identities', 'Site Identities'),
			services: [],
			hosts: hosts.map(member)
		});
		expect(elementShapeKey(result({ inlineGroups: [group(['mv-1'])] }))).not.toBe(
			elementShapeKey(result({ inlineGroups: [group(['mv-1', 'mv-2'])] }))
		);
	});

	it('separates a collapsed manager box from the same box expanded, and ignores members when collapsed', () => {
		const box = (collapsed: boolean, hosts: number) => ({
			groupId: 'identities',
			collapsed,
			header: service('identities', 'Site Identities'),
			services: [],
			hosts: Array.from({ length: hosts }, (_, i) => ({
				host: { id: `mv-${i}`, display_name: `mv-${i}` },
				services: []
			}))
		});
		const key = (collapsed: boolean, hosts: number) =>
			elementShapeKey(result({ inlineGroups: [box(collapsed, hosts)] }));

		expect(key(true, 3)).not.toBe(key(false, 3));
		// Collapsed, only the header row renders: member count no longer changes the height.
		expect(key(true, 3)).toBe(key(true, 45));
	});

	it('separates a service row carrying port lines from one without', () => {
		expect(elementShapeKey(result({ services: [service('a', 'nginx', 0)] }))).not.toBe(
			elementShapeKey(result({ services: [service('a', 'nginx', 3)] }))
		);
	});

	it('separates a card with a staleness pill from one without', () => {
		// The pill renders in flow, so it adds a row.
		expect(elementShapeKey(result())).not.toBe(
			elementShapeKey(result({}, { staleTag: { label: 'Stale' } } as Partial<ElementRenderResult>))
		);
	});

	it('separates a header long enough to wrap from one that fits', () => {
		expect(elementShapeKey(result({ headerText: 'web-01' }))).not.toBe(
			elementShapeKey(
				result({ headerText: 'a-very-long-hostname-that-will-certainly-wrap-onto-more-lines' })
			)
		);
	});

	it('separates element types', () => {
		expect(elementShapeKey(result({ elementType: 'Host' }))).not.toBe(
			elementShapeKey(result({ elementType: 'Interface' }))
		);
	});

	it('separates cards whose inline blocks are hidden by the user', () => {
		// Hiding service rows collapses content, so the heights differ.
		const shown = result({ services: [service('a', 'nginx')] });
		const hidden = result(
			{ services: [service('a', 'nginx')] },
			{ flags: { ...FLAGS, serviceInlineHidden: true, portInlineHidden: true } }
		);
		expect(elementShapeKey(shown)).not.toBe(elementShapeKey(hidden));
	});

	it('separates a port-status block from its absence', () => {
		expect(elementShapeKey(result({ elementType: 'Interface' }))).not.toBe(
			elementShapeKey(
				result({
					elementType: 'Interface',
					portStatus: { operStatus: 1, speed: '1G', macAddress: null }
				})
			)
		);
	});

	it('collapses to a single key for a null render result', () => {
		expect(
			elementShapeKey({ data: null, flags: FLAGS, staleTag: null } as ElementRenderResult)
		).toBe('null');
	});
});
