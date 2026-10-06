import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { FEATURES } from './entity-tabs';
import {
	adjacentEntityId,
	entityListOrderFor,
	registerEntityList
} from '$lib/shared/stores/modal-registry';
import { isEditableTarget } from '$lib/shared/utils/shortcuts';

const order = ['aaa', 'bbb', 'ccc'];

describe('adjacentEntityId', () => {
	it('steps forward and back through the list order', () => {
		expect(adjacentEntityId(order, 'bbb', 1)).toBe('ccc');
		expect(adjacentEntityId(order, 'bbb', -1)).toBe('aaa');
	});

	it('stops at both ends instead of wrapping', () => {
		expect(adjacentEntityId(order, 'aaa', -1)).toBeNull();
		expect(adjacentEntityId(order, 'ccc', 1)).toBeNull();
	});

	it('goes nowhere for an entity outside the list or with no list', () => {
		expect(adjacentEntityId(order, 'zzz', 1)).toBeNull();
		expect(adjacentEntityId(null, 'aaa', 1)).toBeNull();
	});
});

describe('entityListOrderFor', () => {
	it('uses the visible list that holds the entity', () => {
		const offscreen = registerEntityList({ ids: () => ['bbb', 'aaa'], isVisible: () => false });
		const onscreen = registerEntityList({ ids: () => order, isVisible: () => true });
		const other = registerEntityList({ ids: () => ['xxx'], isVisible: () => true });
		try {
			expect(entityListOrderFor('tag-editor', 'bbb')).toEqual(order);
			expect(entityListOrderFor('tag-editor', 'yyy')).toBeNull();
		} finally {
			offscreen();
			onscreen();
			other();
		}
	});

	it('has no order for a dialog that is not an entity modal', () => {
		const unregister = registerEntityList({ ids: () => order, isVisible: () => true });
		try {
			expect(entityListOrderFor('provisional-range', 'bbb')).toBeNull();
			expect(entityListOrderFor('discovery-history-detail', 'bbb')).toEqual(order);
		} finally {
			unregister();
		}
	});

	it('forgets a list once it unregisters', () => {
		const unregister = registerEntityList({ ids: () => order, isVisible: () => true });
		unregister();
		expect(entityListOrderFor('tag-editor', 'aaa')).toBeNull();
	});
});

describe('isEditableTarget', () => {
	const el = (tagName: string, isContentEditable = false) =>
		({ tagName, isContentEditable }) as unknown as EventTarget;

	it('is true for text-editing elements, so arrows still move the cursor', () => {
		for (const tag of ['INPUT', 'TEXTAREA', 'SELECT']) expect(isEditableTarget(el(tag))).toBe(true);
		expect(isEditableTarget(el('DIV', true))).toBe(true);
	});

	it('is false for everything else', () => {
		expect(isEditableTarget(el('BUTTON'))).toBe(false);
		expect(isEditableTarget(null)).toBe(false);
		expect(isEditableTarget({} as EventTarget)).toBe(false);
	});
});

/**
 * Arrow keys step from one entity to the next only after asking about unsaved changes, and the
 * modal can only ask about a form it was handed. Every entity modal that builds a form passes it.
 */
describe('entity modals hand their form to GenericModal', () => {
	const svelteFiles = (dir: string): string[] =>
		fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
			const full = path.join(dir, entry.name);
			if (entry.isDirectory()) return svelteFiles(full);
			return entry.name.endsWith('.svelte') ? [full] : [];
		});

	const entityModals = svelteFiles(FEATURES).flatMap((file) => {
		const source = fs.readFileSync(file, 'utf8');
		if (!/create\w*Form\(/.test(source)) return [];
		const tags = source.match(/<GenericModal\b[\s\S]*?\n\t*>/g) ?? [];
		return tags
			.filter((tag) => /\bentityId=/.test(tag))
			.map((tag) => ({ file: path.relative(FEATURES, file), tag }));
	});
	const offenders = entityModals
		.filter(({ tag }) => !/(\{form\}|\bform=)/.test(tag))
		.map(({ file }) => file);

	it('finds the entity modals', () => {
		// Host, service, subnet, site, tag, VLAN, credential, discovery, daemon, user and both API keys.
		expect(entityModals.length).toBeGreaterThanOrEqual(12);
	});

	it('passes the form wherever the modal has an entity to step from', () => {
		expect(offenders).toEqual([]);
	});
});
