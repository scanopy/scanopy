import { get, writable } from 'svelte/store';
import type { EntityDiscriminants } from '$lib/api/entities';
import { entityUIConfig, TAB_LABELS } from '$lib/shared/entity-ui-config';

export interface ModalState {
	name: string | null;
	id: string | null;
	tab: string | null;
	subEntityId: string | null;
	returnUrl: string | null;
	returnTitle: string | null;
	entityData?: Record<string, unknown>;
}

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
	}
): void {
	const state: ModalState = {
		name,
		id: opts?.id ?? null,
		tab: opts?.tab ?? null,
		subEntityId: opts?.subEntityId ?? null,
		returnUrl: opts?.returnUrl ?? null,
		returnTitle: opts?.returnTitle ?? null,
		entityData: opts?.entityData
	};
	modalState.set(state);
	syncToUrl(state);
}

/**
 * Close the current modal. Clears the store and URL search params.
 */
export function closeModal(): void {
	modalState.set({ ...EMPTY_STATE });
	syncToUrl(EMPTY_STATE);
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
	syncToUrl(state);
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
		syncToUrl(next);
		return next;
	});
}

/**
 * Read URL params into the modal store. Call once after app initialization.
 */
export function initModalFromUrl(): void {
	if (typeof window === 'undefined') return;
	const params = new URLSearchParams(window.location.search);
	const name = params.get('modal');
	if (!name) return;
	const state: ModalState = {
		name,
		id: params.get('id'),
		tab: params.get('tab'),
		subEntityId: params.get('subEntityId'),
		returnUrl: null,
		returnTitle: null
	};
	modalState.set(state);
}

/**
 * Navigate to an entity's tab and open its edit modal.
 * For sub-entities (Interface, Interface, etc.), opens the parent Host's modal on the relevant tab.
 */
export function navigateToEntity(
	entityType: EntityDiscriminants,
	entityId: string,
	data?: Record<string, unknown>
): void {
	const typeConfig = entityUIConfig[entityType];
	const config = (data && typeConfig?.forEntity?.(data)) || typeConfig;
	if (!config) return;

	// Snapshot current URL and modal title before navigation so the back button can return here
	const returnUrl = typeof window !== 'undefined' ? window.location.href : undefined;
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

function syncToUrl(state: ModalState): void {
	if (typeof window === 'undefined') return;
	const url = new URL(window.location.href);
	if (state.name) {
		url.searchParams.set('modal', state.name);
		if (state.id) {
			url.searchParams.set('id', state.id);
		} else {
			url.searchParams.delete('id');
		}
		if (state.tab) {
			url.searchParams.set('tab', state.tab);
		} else {
			url.searchParams.delete('tab');
		}
		if (state.subEntityId) {
			url.searchParams.set('subEntityId', state.subEntityId);
		} else {
			url.searchParams.delete('subEntityId');
		}
	} else {
		url.searchParams.delete('modal');
		url.searchParams.delete('id');
		url.searchParams.delete('tab');
		url.searchParams.delete('subEntityId');
	}
	window.history.replaceState({}, '', url.toString());
}
