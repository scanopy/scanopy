import type { components } from '#lib/api/schema';
import { entities } from '#lib/shared/stores/metadata.ts';
import { getTopologyIndex } from './entity-index';
import type { RenderableTopology, TopologyHost } from './types/base';

type DeviceType = components['schemas']['DeviceType'];

/**
 * What an entity draws as its glyph: a device as its host's device type (a square tile), software as its
 * definition's logo (a round frame), a port as its entity icon heading a one-line row. Which entity is
 * which is the backend's `glyph` metadata on the entity type; this only resolves the device type or
 * definition for the one entity in hand.
 */
export type EntityGlyph =
	| { type: 'device'; deviceType: DeviceType }
	| { type: 'software'; definition: string }
	| { type: 'port' };

/** The glyph for an entity of a given type, or null for a type that keeps its plain icon. */
export function entityGlyph(
	entityType: string,
	entity: object,
	topology: RenderableTopology
): EntityGlyph | null {
	const meta = entities.getMetadata(entityType) as { glyph?: string } | null;
	switch (meta?.glyph) {
		case 'Device': {
			// A device glyph is its machine's device type: the host itself, or the host an address sits on.
			const hostId =
				'host_id' in entity ? (entity.host_id as string) : (entity as { id: string }).id;
			const host = getTopologyIndex(topology).hostsById.get(hostId) as TopologyHost | undefined;
			return { type: 'device', deviceType: host?.device_type ?? 'Unidentified' };
		}
		case 'Software':
			return {
				type: 'software',
				definition: (entity as { service_definition: string }).service_definition
			};
		case 'Port':
			return { type: 'port' };
		default:
			return null;
	}
}
