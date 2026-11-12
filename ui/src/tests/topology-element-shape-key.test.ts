import { describe, it, expect } from 'vitest';
import {
	elementShapeKey,
	type ElementIconCard,
	type ElementRenderResult,
	type IconCardGroup
} from '#lib/features/topology/element-render-data.ts';

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
	inlineEntities: ['Service'],
	inlinesService: true,
	serviceInlineHidden: false
};

const STALE = { staleTag: { label: 'Stale' } } as Partial<ElementRenderResult>;

function iconCard(overrides: Partial<ElementIconCard> = {}): ElementIconCard {
	return {
		glyph: { type: 'device', deviceType: 'Server' },
		badgeDefinitions: [],
		label: 'web-01',
		sublabel: null,
		os: null,
		tooltip: () => ({ title: 'web-01', sections: [] }),
		groups: [],
		...overrides
	};
}

function result(
	card: ElementIconCard | null,
	elementType = 'Host',
	extra: Partial<ElementRenderResult> = {}
): ElementRenderResult {
	return {
		flags: FLAGS,
		staleTag: null,
		marks: {},
		glyph: card?.glyph ?? null,
		iconCard: card,
		data: {
			elementType,
			headerText: 'web-01',
			bodyText: card ? null : 'Unknown Service',
			showServices: false,
			services: [],
			ip_address_id: '',
			inlineGroups: []
		},
		...extra
	} as ElementRenderResult;
}

function group(collapsed: boolean, members: number): IconCardGroup {
	return {
		groupId: 'docker',
		headerDefinition: 'Docker',
		headerName: 'Docker',
		collapsed,
		members: Array.from({ length: members }, (_, i) => ({
			id: `c-${i}`,
			glyph: { type: 'software', definition: 'Nginx' },
			label: `container-${i}`,
			status: undefined,
			tooltip: () => ({ title: `container-${i}`, sections: [] })
		}))
	};
}

const port = (label: string) => iconCard({ glyph: { type: 'port' }, label });

describe('elementShapeKey', () => {
	it('matches icon cards that render identically but describe different entities', () => {
		expect(elementShapeKey(result(iconCard({ label: 'web-01' })))).toBe(
			elementShapeKey(result(iconCard({ label: 'web-02' })))
		);
	});

	it('separates a label long enough to wrap from one that fits', () => {
		expect(elementShapeKey(result(iconCard({ label: 'web-01' })))).not.toBe(
			elementShapeKey(result(iconCard({ label: 'a-hostname-long-enough-to-wrap' })))
		);
	});

	it('separates an address card with a hostname line from one without', () => {
		expect(elementShapeKey(result(iconCard({ label: '192.168.1.10' })))).not.toBe(
			elementShapeKey(result(iconCard({ label: '192.168.1.10', sublabel: 'web-01.lan' })))
		);
	});

	it('separates a card with a staleness pill from one without', () => {
		// The pill renders in flow, so it adds a row: on a device card and on a port card alike.
		expect(elementShapeKey(result(iconCard()))).not.toBe(
			elementShapeKey(result(iconCard(), 'Host', STALE))
		);
		expect(elementShapeKey(result(port('Gi1/0/1'), 'Interface'))).not.toBe(
			elementShapeKey(result(port('Gi1/0/1'), 'Interface', STALE))
		);
	});

	it('separates a port name long enough to wrap from one that fits', () => {
		expect(elementShapeKey(result(port('Gi1/0/1'), 'Interface'))).not.toBe(
			elementShapeKey(result(port('TenGigabitEthernet1/1/48 uplink to core'), 'Interface'))
		);
	});

	it('separates manager boxes holding a different number of open members', () => {
		expect(elementShapeKey(result(iconCard({ groups: [group(false, 1)] })))).not.toBe(
			elementShapeKey(result(iconCard({ groups: [group(false, 2)] })))
		);
	});

	it('separates a collapsed manager box from the same box open, and ignores members when collapsed', () => {
		const key = (collapsed: boolean, members: number) =>
			elementShapeKey(result(iconCard({ groups: [group(collapsed, members)] })));

		expect(key(true, 3)).not.toBe(key(false, 3));
		// Collapsed, only the heading renders: the member count no longer changes the height.
		expect(key(true, 3)).toBe(key(true, 45));
	});

	it('separates element types', () => {
		expect(elementShapeKey(result(iconCard(), 'Host'))).not.toBe(
			elementShapeKey(result(iconCard(), 'IPAddress'))
		);
	});

	it('separates the text fallback from an icon card', () => {
		expect(elementShapeKey(result(null, 'Service'))).not.toBe(
			elementShapeKey(
				result(iconCard({ glyph: { type: 'software', definition: 'Nginx' } }), 'Service')
			)
		);
	});

	it('collapses to a single key for a null render result', () => {
		expect(
			elementShapeKey({ data: null, flags: FLAGS, staleTag: null } as ElementRenderResult)
		).toBe('null');
	});
});
