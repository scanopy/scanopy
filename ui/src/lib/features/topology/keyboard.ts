import { get } from 'svelte/store';
import type { Node } from '@xyflow/svelte';
import { searchOpen, clearSearch } from './interactions';
import { clearSelection, type SelectionStores } from './selection';
import type BaseTopologyViewer from './components/visualization/BaseTopologyViewer.svelte';
import { isFindShortcut } from '$lib/features/search/results';

export interface KeyboardShortcutHandlers {
	getBaseViewer: () => BaseTopologyViewer | null;
	getShortcutsHelpOpen: () => boolean;
	setShortcutsHelpOpen: (open: boolean) => void;
	selectionStores: SelectionStores;
	/** Guard — return false to skip all shortcuts (e.g. tab not active) */
	isEnabled?: () => boolean;
}

function isInputElement(target: EventTarget | null): boolean {
	if (!target || !(target instanceof HTMLElement)) return false;
	const tag = target.tagName.toLowerCase();
	if (tag === 'input' || tag === 'textarea' || tag === 'select') return true;
	if (target.isContentEditable) return true;
	return false;
}

/**
 * Create a keydown handler for topology keyboard shortcuts.
 * Used by both the main TopologyViewer and the read-only share viewer.
 */
export function createTopologyKeydownHandler(handlers: KeyboardShortcutHandlers) {
	return function handleKeydown(event: KeyboardEvent) {
		if (handlers.isEnabled && !handlers.isEnabled()) return;
		if (handlers.getShortcutsHelpOpen()) return;

		const isSearchOpen = get(searchOpen);

		// Escape always works
		if (event.key === 'Escape') {
			if (isSearchOpen) {
				clearSearch();
			} else {
				clearSelection(handlers.selectionStores);
			}
			return;
		}

		// Skip shortcuts when typing in inputs (except Escape handled above)
		if (isInputElement(event.target)) return;

		if (isFindShortcut(event)) {
			event.preventDefault();
			searchOpen.set(true);
			return;
		}

		// Single key shortcuts (no modifiers)
		if (event.metaKey || event.ctrlKey || event.altKey) return;

		const viewer = handlers.getBaseViewer();

		switch (event.key) {
			case 'f':
			case 'F':
				viewer?.triggerFitView();
				break;
			case 'z':
			case 'Z': {
				const multiSelected = get(handlers.selectionStores.selectedNodes);
				const singleSelected = get(handlers.selectionStores.selectedNode);
				if (multiSelected.length >= 2) {
					viewer?.fitViewToNodes(multiSelected.map((n: Node) => n.id));
				} else if (singleSelected) {
					viewer?.fitViewToNodes([singleSelected.id]);
				}
				break;
			}
			case '?':
				handlers.setShortcutsHelpOpen(!handlers.getShortcutsHelpOpen());
				break;
			case ']':
				viewer?.triggerStepExpand();
				break;
			case '[':
				viewer?.triggerStepCollapse();
				break;
		}
	};
}
