import { describe, it, expect } from 'vitest';
import {
	flattenGroups,
	moveHighlight,
	isGlobalSearchShortcut,
	shortcutLabel
} from '$lib/features/search/results';

const key = (k: string, mods: Partial<KeyboardEvent> = {}) => ({
	key: k,
	metaKey: false,
	ctrlKey: false,
	altKey: false,
	shiftKey: false,
	...mods
});

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
	it('accepts Cmd+K and Ctrl+K, in either case', () => {
		expect(isGlobalSearchShortcut(key('k', { metaKey: true }))).toBe(true);
		expect(isGlobalSearchShortcut(key('K', { ctrlKey: true }))).toBe(true);
	});

	it('leaves bare K and Shift/Alt chords alone', () => {
		expect(isGlobalSearchShortcut(key('k'))).toBe(false);
		expect(isGlobalSearchShortcut(key('k', { metaKey: true, shiftKey: true }))).toBe(false);
		expect(isGlobalSearchShortcut(key('k', { ctrlKey: true, altKey: true }))).toBe(false);
	});

	it('does not claim the topology search shortcut', () => {
		expect(isGlobalSearchShortcut(key('f', { metaKey: true }))).toBe(false);
	});
});

describe('shortcutLabel', () => {
	it('uses the Command glyph only on Apple platforms', () => {
		expect(shortcutLabel('MacIntel')).toBe('⌘K');
		expect(shortcutLabel('Win32')).toBe('Ctrl K');
		expect(shortcutLabel('Linux x86_64')).toBe('Ctrl K');
	});
});
