import type { components } from '#lib/api/schema';
import { views } from '#lib/shared/stores/metadata.ts';

export type ViewLayoutConfig = components['schemas']['ViewLayoutConfig'];

/**
 * Layout for a view the views fixture has no layout for: the default algorithm with no
 * view-specific behaviour. Only reachable with fixtures older than the backend's layout config.
 */
const FALLBACK_LAYOUT: ViewLayoutConfig = {
	algorithm: 'default',
	handle_strategy: 'optimal',
	element_order: 'automatic',
	root_order: 'input',
	root_edges: true,
	repack_disconnected: true,
	align_layers_top: false,
	force_overview: true
};

/** How the layout pipeline lays out a view, from the backend's view metadata. */
export function getViewLayout(view: string): ViewLayoutConfig {
	return views.getMetadata(view).layout ?? FALLBACK_LAYOUT;
}
