import type { components } from '$lib/api/schema';

// Re-export generated types
export type EntitySource = components['schemas']['EntitySource'];
export type DiscoveryType = components['schemas']['DiscoveryType'];
export type MatchDetails = components['schemas']['MatchDetails'];
export type MatchConfidence = components['schemas']['MatchConfidence'];
export type MatchReason = components['schemas']['MatchReason'];
export type HostNamingFallback = components['schemas']['HostNamingFallback'];

// Frontend-specific types
export interface GetAllEntitiesRequest {
	network_id: string;
}

// Shared props interface for sidebar tab components
export interface TabProps {
	isReadOnly?: boolean;
	isActive?: boolean;
}

/** Get a display string for a MatchReason */
export function matchReasonLabel(reason: MatchReason): string {
	if (reason.type === 'reason') {
		return reason.data;
	} else {
		// Container type: [name, children] - data is typed as unknown[] in schema
		return reason.data[0] as string;
	}
}
