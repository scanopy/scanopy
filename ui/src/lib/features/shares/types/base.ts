// Re-export generated types from OpenAPI schema
import type { components } from '$lib/api/schema';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Share = components['schemas']['Share'];
export type ShareOptions = components['schemas']['ShareOptions'];
export type CreateUpdateShareRequest = components['schemas']['CreateUpdateShareRequest'];
export type PublicShareMetadata = components['schemas']['PublicShareMetadata'];
export type TopologyData = components['schemas']['TopologyData'];

export type ExportFeatures = components['schemas']['ExportFeatures'];
export type TopologyView = components['schemas']['TopologyView'];

// Share metadata plus the slim topology row and the TopologyData bundle
// (entities + per-view graph built on request). The share viewer composes these
// into a RenderableTopology with the same `toRenderableTopology` the app uses —
// shared users can't load entities via the per-entity endpoints with their own
// credentials, so the bundle ships them.
export type ShareWithTopology = components['schemas']['ShareWithTopology'];

export const defaultShareOptions: ShareOptions = {
	show_inspect_panel: true,
	show_zoom_controls: true,
	show_export_button: true,
	show_minimap: true
};

export function createEmptyShare(topology_id: string, network_id: string): Share {
	return {
		topology_id,
		network_id,
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel,
		created_by: uuidv4Sentinel,
		expires_at: null,
		allowed_domains: null,
		name: '',
		is_enabled: true,
		options: { ...defaultShareOptions },
		enabled_views: null
	};
}
