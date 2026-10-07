import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import {
	acceptCompletion,
	backspaceChip,
	flattenGroups,
	hasSearchTerms,
	moveHighlight,
	isFindShortcut,
	isGlobalSearchShortcut,
	removeChip,
	responseGroups,
	tagCompletion
} from '$lib/features/search/results';
import type { Tag } from '$lib/features/tags/types/base';
import { entityUIConfig } from '$lib/shared/entity-ui-config';
import { FEATURES } from './entity-tabs';
import { keyLabel, shortcutLabel } from '$lib/shared/utils/shortcuts';

const key = (
	k: string,
	mods: Partial<Pick<KeyboardEvent, 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>> = {},
	target: EventTarget | null = null
) => ({
	key: k,
	metaKey: false,
	ctrlKey: false,
	altKey: false,
	shiftKey: false,
	target,
	...mods
});

/** An element as the event target sees it: only the parts the shortcut checks read. */
const element = (tagName: string, isContentEditable = false) =>
	({ tagName, isContentEditable }) as unknown as EventTarget;

describe('flattenGroups', () => {
	it('keeps section order, drops empty sections, and tags each row with its type', () => {
		const rows = flattenGroups([
			{ type: 'Host', items: ['h1', 'h2'] },
			{ type: 'Service', items: [] },
			{ type: 'Subnet', items: ['s1'] }
		]);
		expect(rows).toEqual([
			{ type: 'Host', item: 'h1' },
			{ type: 'Host', item: 'h2' },
			{ type: 'Subnet', item: 's1' }
		]);
	});
});

describe('moveHighlight', () => {
	it('wraps past both ends', () => {
		expect(moveHighlight(2, 1, 3)).toBe(0);
		expect(moveHighlight(0, -1, 3)).toBe(2);
	});

	it('enters the list from nothing highlighted at the end the key points to', () => {
		expect(moveHighlight(-1, 1, 3)).toBe(0);
		expect(moveHighlight(-1, -1, 3)).toBe(2);
	});

	it('highlights nothing when there are no results', () => {
		expect(moveHighlight(0, 1, 0)).toBe(-1);
	});
});

describe('isGlobalSearchShortcut', () => {
	it('opens on a bare slash', () => {
		expect(isGlobalSearchShortcut(key('/'))).toBe(true);
		expect(isGlobalSearchShortcut(key('/', {}, element('DIV')))).toBe(true);
	});

	it('leaves a slash typed into a field alone', () => {
		for (const tag of ['INPUT', 'TEXTAREA', 'SELECT']) {
			expect(isGlobalSearchShortcut(key('/', {}, element(tag)))).toBe(false);
		}
		expect(isGlobalSearchShortcut(key('/', {}, element('DIV', true)))).toBe(false);
	});

	it('leaves modified slashes to the browser', () => {
		expect(isGlobalSearchShortcut(key('/', { metaKey: true }))).toBe(false);
		expect(isGlobalSearchShortcut(key('/', { ctrlKey: true }))).toBe(false);
	});
});

describe('isFindShortcut', () => {
	it('accepts Cmd+F and Ctrl+F, in either case', () => {
		expect(isFindShortcut(key('f', { metaKey: true }))).toBe(true);
		expect(isFindShortcut(key('F', { ctrlKey: true }))).toBe(true);
	});

	it('leaves bare F and Shift/Alt chords alone', () => {
		expect(isFindShortcut(key('f'))).toBe(false);
		expect(isFindShortcut(key('f', { metaKey: true, shiftKey: true }))).toBe(false);
		expect(isFindShortcut(key('f', { ctrlKey: true, altKey: true }))).toBe(false);
	});
});

describe('shortcutLabel', () => {
	it('uses the Command glyph only on Apple platforms', () => {
		expect(shortcutLabel('F', 'MacIntel')).toBe('⌘F');
		expect(shortcutLabel('F', 'Win32')).toBe('Ctrl F');
		expect(shortcutLabel('F', 'Linux x86_64')).toBe('Ctrl F');
	});

	it('spells modifiers per platform and passes other keys through', () => {
		expect(keyLabel('Shift', 'iPhone')).toBe('⇧');
		expect(keyLabel('Shift', 'Win32')).toBe('Shift');
		expect(keyLabel('[', 'MacIntel')).toBe('[');
	});
});

const tag = (name: string, id = name) => ({ id, name }) as Tag;

describe('tagCompletion', () => {
	const tags = [tag('production'), tag('prod'), tag('printers'), tag('Payments')];

	it('completes text that starts a tag name, ignoring case and surrounding space', () => {
		expect(tagCompletion('  PRI ', tags, [])?.name).toBe('printers');
		expect(tagCompletion('pay', tags, [])?.name).toBe('Payments');
	});

	it('prefers an exact name, then the shortest, then the alphabetically first', () => {
		expect(tagCompletion('prod', tags, [])?.name).toBe('prod');
		expect(tagCompletion('pr', tags, [])?.name).toBe('prod');
		expect(tagCompletion('p', [tag('pb'), tag('pa')], [])?.name).toBe('pa');
	});

	it('skips tags already chipped', () => {
		expect(tagCompletion('prod', tags, ['prod'])?.name).toBe('production');
	});

	it('offers nothing for empty text or text inside a name', () => {
		expect(tagCompletion('  ', tags, [])).toBeNull();
		expect(tagCompletion('duct', tags, [])).toBeNull();
	});
});

describe('tag chips', () => {
	it('accepting a completion adds its chip and clears the text it completed', () => {
		expect(acceptCompletion({ text: 'pro', tagIds: ['a'] }, tag('production', 'b'))).toEqual({
			text: '',
			tagIds: ['a', 'b']
		});
	});

	it("a chip's x removes that chip and keeps the text", () => {
		expect(removeChip({ text: 'web', tagIds: ['a', 'b', 'c'] }, 'b')).toEqual({
			text: 'web',
			tagIds: ['a', 'c']
		});
	});

	it('Backspace on an empty input removes the last chip, and otherwise leaves the key alone', () => {
		expect(backspaceChip({ text: '', tagIds: ['a', 'b'] })).toEqual({ text: '', tagIds: ['a'] });
		expect(backspaceChip({ text: 'w', tagIds: ['a'] })).toBeNull();
		expect(backspaceChip({ text: '', tagIds: [] })).toBeNull();
	});

	it('chips alone are something to search for', () => {
		expect(hasSearchTerms({ text: ' ', tagIds: [] })).toBe(false);
		expect(hasSearchTerms({ text: '', tagIds: ['a'] })).toBe(true);
	});
});

describe('responseGroups', () => {
	it('unwraps each match from its entity variant, keeping the server order', () => {
		const host = { id: 'h1' };
		const vlan = { id: 'v1' };
		const groups = responseGroups({
			groups: [
				{ entity_type: 'Vlan', items: [{ Vlan: vlan }] },
				{ entity_type: 'Host', items: [{ Host: host }, 'Unknown'] }
			]
		} as unknown as Parameters<typeof responseGroups>[0]);
		expect(groups).toEqual([
			{ type: 'Vlan', items: [vlan] },
			{ type: 'Host', items: [host] }
		]);
	});
});

/**
 * A search result opens through `navigateToEntity`, which opens the type's `modalName`. That only
 * works when some tab resolves that modal from the URL, and the row only renders with a display.
 */
describe('every entity modal a search result can open', () => {
	const sources = (dir: string): string[] =>
		fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
			const full = path.join(dir, entry.name);
			if (entry.isDirectory()) return sources(full);
			return entry.name.endsWith('.svelte') ? [fs.readFileSync(full, 'utf8')] : [];
		});
	const resolved = new Set(
		sources(FEATURES).flatMap((source) =>
			[...source.matchAll(/resolveModalDeepLink\(\s*[^,]+,\s*'([^']+)'/g)].map((m) => m[1])
		)
	);
	const configs = Object.entries(entityUIConfig).flatMap(([type, config]) =>
		config ? [config, ...(config.variants ?? [])].map((c) => ({ type, ...c })) : []
	);
	const modals = configs.filter((config) => config.modalName);

	it('is resolved by a tab, so opening it from a URL shows it', () => {
		expect(modals.filter((c) => !resolved.has(c.modalName!)).map((c) => c.modalName)).toEqual([]);
	});

	it('has a display, so its search row renders', () => {
		const ownModals = modals.filter((c) => configs.find((o) => o.type === c.type) === c);
		expect(ownModals.filter((c) => !c.displayComponent).map((c) => c.type)).toEqual([]);
	});
});
