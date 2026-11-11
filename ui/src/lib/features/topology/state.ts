import { writable } from 'svelte/store';
import type { Topology, RenderableTopology } from './types/base';
import { topology_optionDisabledRebuildRequired } from '#lib/paraglide/messages.js';

/** Whether edit mode is active (nodes draggable, containers resizable). */
export const editModeEnabled = writable(false);

export interface TopologyEditState {
	isReadonly: boolean;
	isEditable: boolean;
	disabledReason: 'readonly' | null;
}

/**
 * Determine the edit state for a topology. After the snapshot refactor the
 * lock/staleness model is gone — the only reason a topology is non-editable
 * is the read-only context (share/embed view). The third parameter is kept
 * for call-site compatibility with the previous lock/staleness signature.
 */
export function getTopologyEditState(
	topology: Topology | RenderableTopology | null | undefined,
	_autoRebuild: boolean,
	isReadonly: boolean
): TopologyEditState {
	if (isReadonly) return { isReadonly: true, isEditable: false, disabledReason: 'readonly' };
	if (!topology) return { isReadonly: false, isEditable: false, disabledReason: null };
	return { isReadonly: false, isEditable: true, disabledReason: null };
}

export function getOptionDisabledTooltip(): string {
	return topology_optionDisabledRebuildRequired();
}
