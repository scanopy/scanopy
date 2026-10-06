import { describe, it, expect } from 'vitest';
import {
	flattenGroups,
	moveHighlight,
	isFindShortcut,
	isGlobalSearchShortcut,
	shortcutLabel
} from '$lib/features/search/results';

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
		expect(shortcutLabel('MacIntel', 'F')).toBe('⌘F');
		expect(shortcutLabel('Win32', 'F')).toBe('Ctrl F');
		expect(shortcutLabel('Linux x86_64', 'F')).toBe('Ctrl F');
	});
});
