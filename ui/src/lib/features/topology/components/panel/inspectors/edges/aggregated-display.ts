import type { components } from '#lib/api/schema';
import type { RenderableTopology, TopologyEdge } from '#lib/features/topology/types/base.ts';
import type { EntityDisplayComponent } from '#lib/shared/components/forms/selection/types.ts';
import { SameHostEdgeDisplay } from '#lib/shared/components/forms/selection/display/SameHostEdgeDisplay.svelte';
import { PhysicalLinkEdgeDisplay } from '#lib/shared/components/forms/selection/display/PhysicalLinkEdgeDisplay.svelte';
import { HypervisorEdgeDisplay } from '#lib/shared/components/forms/selection/display/HypervisorEdgeDisplay.svelte';
import { ContainerRuntimeEdgeDisplay } from '#lib/shared/components/forms/selection/display/ContainerRuntimeEdgeDisplay.svelte';

type EdgeType = components['schemas']['EdgeTypeDiscriminants'];
export type AggregatedEdgeDisplay = EntityDisplayComponent<
	TopologyEdge,
	{ topology?: RenderableTopology }
>;

/**
 * The card each edge of an aggregated connection draws, by edge type, for types the aggregated
 * inspector doesn't group in its own way. `null` draws the type name alone. Keyed by every edge
 * type, so a new one fails to compile until it is given a display or an explicit `null`.
 */
export const AGGREGATED_DISPLAY = {
	SameHost: SameHostEdgeDisplay,
	PhysicalLink: PhysicalLinkEdgeDisplay,
	NeighborLink: PhysicalLinkEdgeDisplay,
	Hypervisor: HypervisorEdgeDisplay,
	ContainerRuntime: ContainerRuntimeEdgeDisplay,
	NetworkIdentity: null,
	SameContainer: null,
	RequestPath: null,
	HubAndSpoke: null
} satisfies Record<EdgeType, AggregatedEdgeDisplay | null>;

export function aggregatedDisplay(edgeType: string): AggregatedEdgeDisplay | null {
	return (AGGREGATED_DISPLAY as Record<string, AggregatedEdgeDisplay | null>)[edgeType] ?? null;
}
