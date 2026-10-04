<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import { getContainerContents, resolveInlineServiceIds } from '$lib/features/topology/resolvers';
	import { containerTypes } from '$lib/shared/stores/metadata';
	import { inspector_crossContainerDeps, inspector_noDependencies } from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import InspectorSubsection from '../shared/InspectorSubsection.svelte';
	import EntityDisplayWrapper from '$lib/shared/components/forms/selection/display/EntityDisplayWrapper.svelte';
	import { DependencyDisplay } from '$lib/shared/components/forms/selection/display/DependencyDisplay.svelte';

	let {
		node,
		topology
	}: {
		node: Node;
		topology: RenderableTopology;
	} = $props();

	// Container-type-aware label: "Cross-Application Dependencies" / "Cross-Host Dependencies" / etc.
	let containerTypeName = $derived.by((): string => {
		const ct = (node.data as { container_type?: string } | undefined)?.container_type;
		if (!ct) return '';
		return containerTypes.getName(ct) || ct;
	});

	// Find all element and inline entities in this container (including subcontainers).
	// Inline services (rendered inside host elements in Workloads, or IP-address
	// elements in L3) must count as "inside" for the cross-boundary check — without
	// this union, dependencies involving inlined services look mis-classified.
	let descendantEntityIds = $derived.by(() => {
		const c = getContainerContents(node.id, topology.nodes);
		const inlineServices = resolveInlineServiceIds(c.elementNodeIds, topology);
		return new Set<string>([...c.elementNodeIds, ...inlineServices]);
	});

	// Find dependencies that cross this container boundary
	// (have members both inside and outside)
	let crossBoundaryDeps = $derived.by(() => {
		const childSet = descendantEntityIds;
		return topology.dependencies.filter((d) => {
			const members = d.members;
			let memberServiceIds: string[] = [];
			if (members.type === 'Services') {
				memberServiceIds = members.service_ids;
			} else if (members.type === 'Bindings') {
				memberServiceIds = members.binding_ids
					.map((bid) => {
						const svc = topology.services.find((s) => s.bindings.some((b) => b.id === bid));
						return svc?.id ?? '';
					})
					.filter(Boolean);
			}
			const hasInside = memberServiceIds.some((id) => childSet.has(id));
			const hasOutside = memberServiceIds.some((id) => !childSet.has(id));
			return hasInside && hasOutside;
		});
	});
</script>

<InspectorSection
	id="DependencySummary"
	section="DependencySummary"
	count={crossBoundaryDeps.length > 0 ? crossBoundaryDeps.length : undefined}
>
	{#if crossBoundaryDeps.length === 0}
		<p class="text-tertiary text-sm">{inspector_noDependencies()}</p>
	{:else}
		<InspectorSubsection
			id="DependencySummary:crossing"
			title={inspector_crossContainerDeps({ containerType: containerTypeName })}
			count={crossBoundaryDeps.length}
		>
			<div class="space-y-1">
				{#each crossBoundaryDeps as dep (dep.id)}
					<div class="card card-static">
						<EntityDisplayWrapper
							item={dep}
							context={{ compact: true }}
							displayComponent={DependencyDisplay}
						/>
					</div>
				{/each}
			</div>
		</InspectorSubsection>
	{/if}
</InspectorSection>
