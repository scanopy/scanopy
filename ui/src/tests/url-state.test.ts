import { afterEach, beforeEach, describe, it, expect, vi } from 'vitest';
import {
	attachUrlState,
	buildUrl,
	openTab,
	registerUrlOwner,
	scopeKeysIn,
	setActiveTab,
	type UrlOwner
} from '#lib/shared/stores/url-state.ts';

function owner(
	scope: UrlOwner['scope'],
	params: string,
	opts: { active?: boolean; keys?: string[] } = {}
): UrlOwner {
	return {
		scope,
		isActive: () => opts.active ?? true,
		params: () => new URLSearchParams(params),
		keys: opts.keys ? () => opts.keys! : undefined
	};
}

const topology = owner('topology', 'topologyId=t1&view=L3Logical');
const modal = owner('modal', 'modal=host-editor&id=h1');
const hostsList = owner('list', 'site_ids=a&site_ids=b&order_by=name', {
	keys: ['site_ids', 'order_by']
});

function built(href: string, owners: UrlOwner[], tab: string | null = null, stale: string[] = []) {
	return buildUrl(new URL(href), owners, tab, stale);
}

describe('buildUrl', () => {
	it('drops topology params off another tab and keeps them on the topology tab', () => {
		const leaked = 'http://x/?topologyId=t1&view=L3Logical#hosts';
		expect(built(leaked, [topology]).search).toBe('');
		expect(built('http://x/#hosts', [topology], 'topology').searchParams.get('topologyId')).toBe(
			't1'
		);
	});

	it('keeps modal params on every tab', () => {
		const url = built('http://x/#hosts', [modal, topology], 'services');
		expect(url.searchParams.get('modal')).toBe('host-editor');
		expect(url.searchParams.has('topologyId')).toBe(false);
	});

	it("writes only the on-screen list's params, arrays repeated", () => {
		const hidden = owner('list', 'search=old', { active: false, keys: ['search'] });
		const url = built('http://x/?search=old#hosts', [hostsList, hidden]);
		expect(url.searchParams.getAll('site_ids')).toEqual(['a', 'b']);
		expect(url.searchParams.has('search')).toBe(false);
	});

	it('hides the list while the palette is open, and restores it when it closes', () => {
		const palette = owner('search', 'q=nas&tag_ids=t1');
		const open = built('http://x/#hosts', [hostsList, palette]);
		expect(open.searchParams.get('q')).toBe('nas');
		expect(open.searchParams.has('site_ids')).toBe(false);

		const closed = owner('search', 'q=nas', { active: false });
		const after = built(open.href, [hostsList, closed]);
		expect(after.searchParams.has('q')).toBe(false);
		expect(after.searchParams.getAll('site_ids')).toEqual(['a', 'b']);
	});

	it('passes through keys no scope owns, and strips handed-out ones nobody writes', () => {
		const url = built('http://x/?billing_flow=done&removed_field=x#hosts', [hostsList], null, [
			'removed_field'
		]);
		expect(url.searchParams.get('billing_flow')).toBe('done');
		expect(url.searchParams.has('removed_field')).toBe(false);
	});

	it('sets the hash only when given a tab', () => {
		expect(built('http://x/#hosts', []).hash).toBe('#hosts');
		expect(built('http://x/#hosts', [], 'services').hash).toBe('#services');
	});
});

describe('scopeKeysIn', () => {
	it('gives a list every key no other scope claims, shell keys excluded', () => {
		const loaded = new URLSearchParams('modal=x&site_ids=a&error=oops&topologyId=t');
		expect(scopeKeysIn('list', loaded)).toEqual(['site_ids']);
		expect(scopeKeysIn('modal', loaded)).toEqual(['modal']);
	});

	it("gives tag_ids to the palette only when the link carries the palette's marker", () => {
		const palette = new URLSearchParams('q=nas&tag_ids=t1');
		expect(scopeKeysIn('search', palette)).toEqual(['q', 'tag_ids']);
		expect(scopeKeysIn('list', palette)).toEqual([]);

		const list = new URLSearchParams('tag_ids=t1');
		expect(scopeKeysIn('search', list)).toEqual([]);
		expect(scopeKeysIn('list', list)).toEqual(['tag_ids']);
	});
});

describe('openTab', () => {
	let href = 'http://localhost/#home';
	let detach: () => void;
	beforeEach(() => {
		vi.stubGlobal('window', {
			get location() {
				const url = new URL(href);
				return {
					href,
					search: url.search,
					get hash() {
						return url.hash;
					},
					set hash(value: string) {
						url.hash = value;
						href = url.toString();
					}
				};
			},
			history: {
				state: null,
				pushState: (_: unknown, __: string, next: string) => {
					href = next;
				},
				replaceState: (_: unknown, __: string, next: string) => {
					href = next;
				}
			},
			addEventListener: () => {},
			removeEventListener: () => {}
		});
		detach = attachUrlState();
	});
	afterEach(() => {
		detach();
		vi.unstubAllGlobals();
		href = 'http://localhost/#home';
	});

	// Every list tab stays mounted, so the hosts list is registered before the nudge is clicked, and
	// is on screen only once the tab switches.
	it('hands the view to the mounted list on the tab and writes it into the URL', async () => {
		let onScreen = false;
		let view = new URLSearchParams();
		const unregister = registerUrlOwner({
			scope: 'list',
			isActive: () => onScreen,
			params: () => view,
			keys: () => ['sources'],
			restore: (params) => {
				view = new URLSearchParams(params);
			}
		});

		openTab('hosts', new URLSearchParams({ sources: 'Inferred' }));
		onScreen = true;
		setActiveTab('hosts');
		await Promise.resolve();

		const url = new URL(href);
		expect(url.hash).toBe('#hosts');
		expect(url.searchParams.get('sources')).toBe('Inferred');
		unregister();
	});
});
