import { get, writable } from 'svelte/store';
import { SvelteSet } from 'svelte/reactivity';
import type { EntityDiscriminants } from '#lib/api/entities.ts';
import { entityModalNames, entityUIConfig, TAB_LABELS } from '#lib/shared/entity-ui-config.ts';
import {
	globalSearchFromUrl,
	globalSearchUrlParams,
	reopenGlobalSearch,
	type GlobalSearchState
} from '#lib/features/search/results.ts';
import { registerUrlOwner, syncUrl, takeInitialParams, URL_SCOPES } from './url-state';

export interface ModalState {
	name: string | null;
	id: string | null;
	tab: string | null;
	subEntityId: string | null;
	returnUrl: string | null;
	returnTitle: string | null;
	entityData?: Record<string, unknown>;
	/**
	 * Values a create-mode modal opens with, carried in the URL so a link from an email or a
	 * reload opens the same pre-filled form. Keys are limited to {@link PREFILL_PARAMS}.
	 */
	prefill?: Partial<Record<PrefillParam, string>>;
}

/** The URL params a modal can be pre-filled from. */
export const PREFILL_PARAMS = ['integration', 'hosts', 'sites'] as const;
export type PrefillParam = (typeof PREFILL_PARAMS)[number];

const EMPTY_STATE: ModalState = {
	name: null,
	id: null,
	tab: null,
	subEntityId: null,
	returnUrl: null,
	returnTitle: null
};

export const modalState = writable<ModalState>({ ...EMPTY_STATE });

/**
 * Open a modal by name. Updates the store and URL search params.
 */
export function openModal(
	name: string,
	opts?: {
		id?: string;
		tab?: string;
		subEntityId?: string;
		returnUrl?: string;
		returnTitle?: string;
		entityData?: Record<string, unknown>;
		prefill?: Partial<Record<PrefillParam, string>>;
	}
): void {
	const state: ModalState = {
		name,
		id: opts?.id ?? null,
		tab: opts?.tab ?? null,
		subEntityId: opts?.subEntityId ?? null,
		returnUrl: opts?.returnUrl ?? null,
		returnTitle: opts?.returnTitle ?? null,
		entityData: opts?.entityData,
		prefill: opts?.prefill
	};
	modalState.set(state);
	syncUrl();
}

/**
 * Close the current modal. Clears the store and URL search params.
 */
export function closeModal(): void {
	modalState.set({ ...EMPTY_STATE });
	syncUrl();
}

/**
 * Re-apply a modal state that was captured before someone else's close handler ran.
 *
 * Exists for one situation, described in full on `GenericModal`'s superseded-modal effect: opening
 * modal B from inside modal A makes A close, and a close handler that calls `closeModal()`
 * unconditionally — the ordinary shape, and correct when the user dismissed A — discards B along
 * with A. Restoring is the repair.
 *
 * Takes the whole state rather than rebuilding it from parts so a future field on `ModalState`
 * cannot be silently dropped on the way through.
 */
export function restoreModal(state: ModalState): void {
	modalState.set({ ...state });
	syncUrl();
}

/**
 * Navigate back to the URL captured before navigateToEntity was called.
 * Closes the current modal and restores the previous URL (which may re-open a previous modal).
 */
export function goBack(): void {
	const current = get(modalState);
	if (!current.returnUrl) return;
	const target = new URL(current.returnUrl);

	// Set hash (triggers tab reactivity)
	window.location.hash = target.hash || '';

	// Opened from the global search palette: close this modal and reopen the palette on the search it
	// was opened from, which the return URL carries as the palette's own params.
	const returnSearch = globalSearchFromUrl(target.searchParams);
	if (returnSearch) {
		closeModal();
		reopenGlobalSearch(returnSearch);
		return;
	}

	// Restore modal state from return URL, or clear if no modal
	const modalName = target.searchParams.get('modal');
	if (modalName) {
		openModal(modalName, {
			id: target.searchParams.get('id') ?? undefined,
			tab: target.searchParams.get('tab') ?? undefined,
			subEntityId: target.searchParams.get('subEntityId') ?? undefined
		});
	} else {
		closeModal();
	}
}

/**
 * Update the active tab in the current modal. Updates store and URL.
 */
export function setModalTab(tab: string): void {
	modalState.update((s) => {
		const next = { ...s, tab };
		syncUrl();
		return next;
	});
}

/**
 * Read URL params into the modal store. Call once after app initialization.
 */
export function initModalFromUrl(): void {
	if (typeof window === 'undefined') return;
	const params = takeInitialParams('modal');
	const name = params.get('modal');
	if (!name) return;
	// Support was its own modal before it became a Settings tab; older links
	// still name it.
	const isLegacySupport = name === 'support';
	const state: ModalState = {
		name: isLegacySupport ? 'settings' : name,
		id: params.get('id'),
		tab: isLegacySupport ? 'support' : params.get('tab'),
		subEntityId: params.get('subEntityId'),
		returnUrl: null,
		returnTitle: null,
		prefill: prefillFromParams(params)
	};
	modalState.set(state);
}

function prefillFromParams(params: URLSearchParams): ModalState['prefill'] {
	const entries = PREFILL_PARAMS.flatMap((key) => {
		const value = params.get(key);
		return value ? [[key, value] as const] : [];
	});
	return entries.length ? Object.fromEntries(entries) : undefined;
}

/**
 * Navigate to an entity's tab and open its edit modal.
 * For sub-entities (Interface, Interface, etc.), opens the parent Host's modal on the relevant tab.
 */
export function navigateToEntity(
	entityType: EntityDiscriminants,
	entityId: string,
	data?: Record<string, unknown>,
	opts?: {
		/** The global search this was opened from, so the back button reopens the palette on it. */
		returnSearch?: GlobalSearchState;
	}
): void {
	const typeConfig = entityUIConfig[entityType];
	const config = (data && typeConfig?.forEntity?.(data)) || typeConfig;
	if (!config) return;

	// Snapshot current URL and modal title before navigation so the back button can return here
	let returnUrl = typeof window !== 'undefined' ? window.location.href : undefined;
	// The palette's params mark it open. Set from the search the entity was opened from, or dropped so
	// the back button doesn't reopen a palette the entity wasn't opened from.
	if (
		returnUrl &&
		(opts?.returnSearch !== undefined || globalSearchFromUrl(new URL(returnUrl).searchParams))
	) {
		const url = new URL(returnUrl);
		for (const key of URL_SCOPES.search.keys) url.searchParams.delete(key);
		if (opts?.returnSearch) {
			for (const [key, value] of globalSearchUrlParams(opts.returnSearch)) {
				url.searchParams.append(key, value);
			}
		}
		returnUrl = url.toString();
	}
	const returnTitle = captureReturnTitle();

	if (config.modalName) {
		window.location.hash = config.tabId;
		openModal(config.modalName, { id: entityId, returnUrl, returnTitle, entityData: data });
	} else if (config.parentType && config.parentIdField && data) {
		const parentConfig = entityUIConfig[config.parentType];
		const parentId = data[config.parentIdField] as string | undefined;
		if (parentConfig?.modalName && parentId) {
			window.location.hash = parentConfig.tabId;
			openModal(parentConfig.modalName, {
				id: parentId,
				tab: config.modalTab,
				subEntityId: entityId,
				returnUrl,
				returnTitle
			});
		}
	} else {
		// Entity has no modal — just navigate to its tab
		window.location.hash = config.tabId;
	}
}

/**
 * Pure helper for deep-link effects in Tab components.
 * Returns the entity to edit (T), null for create mode, or undefined for no action.
 */
export function resolveModalDeepLink<T extends { id: string }>(
	state: ModalState,
	modalName: string,
	data: T[],
	isOpen: boolean,
	editingId: string | null | undefined,
	validate?: (entity: T) => boolean
): T | null | undefined {
	if (state.name !== modalName) return undefined;

	if (!isOpen) {
		if (state.id) {
			const entity =
				data.find((e) => e.id === state.id) ??
				(state.entityData?.id === state.id ? (state.entityData as T) : undefined);
			if (entity && (!validate || validate(entity))) return entity;
		} else {
			return null; // Create mode
		}
	} else if (state.id && state.id !== editingId) {
		const entity =
			data.find((e) => e.id === state.id) ??
			(state.entityData?.id === state.id ? (state.entityData as T) : undefined);
		if (entity && (!validate || validate(entity))) return entity;
	}

	return undefined;
}

/**
 * Capture a human-readable label for the current view (modal title or tab name).
 * Used by navigateToEntity so the back button can say "Back to Hosts" or "Back to Host foo".
 */
function captureReturnTitle(): string | undefined {
	if (typeof document === 'undefined') return undefined;

	// If a modal is open, use its title (strip "Edit " prefix)
	const modalTitle = document.getElementById('modal-title')?.textContent?.trim();
	if (modalTitle) {
		return modalTitle.replace(/^Edit /, '') || undefined;
	}

	// No modal — derive label from current hash (tab ID)
	const tabId = window.location.hash.replace('#', '');
	return TAB_LABELS[tabId] || undefined;
}

/** The modal's URL params. A modal opens over any tab, so they show on every one. */
function modalParams(state: ModalState): Record<string, string> {
	if (!state.name) return {};
	const params: Record<string, string> = { modal: state.name };
	if (state.id) params.id = state.id;
	if (state.tab) params.tab = state.tab;
	if (state.subEntityId) params.subEntityId = state.subEntityId;
	for (const key of PREFILL_PARAMS) {
		const value = state.prefill?.[key];
		if (value) params[key] = value;
	}
	return params;
}

registerUrlOwner({
	scope: 'modal',
	isActive: () => true,
	params: () => new URLSearchParams(modalParams(get(modalState)))
});

/**
 * An entity list on screen, as its rows render: filtered, sorted, grouped and paginated.
 * `DataControls` registers one per mounted list so an open entity modal can step through it.
 */
export interface EntityListSource {
	ids: () => string[];
	/** False while the list's page is mounted but not the one on screen. */
	isVisible: () => boolean;
}

const entityListSources = new SvelteSet<EntityListSource>();

/** Register a list. Returns the unregister function, for use as an effect teardown. */
export function registerEntityList(source: EntityListSource): () => void {
	entityListSources.add(source);
	return () => entityListSources.delete(source);
}

/**
 * The row order of the on-screen list that contains `entityId`, or null when no visible list
 * holds it (opened from topology, or filtered out of its own list). Entity ids are UUIDs, so the
 * list that contains the id is the list the modal belongs to. Null too when `modalName` is not an
 * entity's own modal: a dialog that acts on one entity, such as resolving a subnet's range, has
 * no neighbour to step to.
 */
export function entityListOrderFor(modalName: string, entityId: string): string[] | null {
	if (!entityModalNames.has(modalName)) return null;
	for (const source of entityListSources) {
		const ids = source.ids();
		if (ids.includes(entityId) && source.isVisible()) return ids;
	}
	return null;
}

/**
 * The id one step from `entityId` in `order`, or null at either end or when `entityId` is not
 * in the list. The ends stop rather than wrap, so holding an arrow key halts on the last row.
 */
export function adjacentEntityId(
	order: string[] | null,
	entityId: string,
	step: -1 | 1
): string | null {
	if (!order) return null;
	const index = order.indexOf(entityId);
	if (index === -1) return null;
	return order[index + step] ?? null;
}

/**
 * Whether `el` sits on the list page on screen. Every list page stays mounted; an inactive one
 * sits in a zero-height, overflow-hidden wrapper, so it still lays out and `offsetParent` can't
 * tell.
 */
export function isOnVisiblePage(el: HTMLElement | null | undefined): boolean {
	for (let node: HTMLElement | null = el ?? null; node; node = node.parentElement) {
		if (node.clientHeight === 0 && getComputedStyle(node).overflow === 'hidden') return false;
	}
	return !!el;
}
