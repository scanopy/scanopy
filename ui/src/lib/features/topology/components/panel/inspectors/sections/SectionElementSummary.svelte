<script lang="ts">
	import type { Node } from '@xyflow/svelte';
	import type { components } from '$lib/api/schema';
	import type { RenderableTopology } from '$lib/features/topology/types/base';
	import { activeView, getInfrastructureRuleIdForTopology } from '$lib/features/topology/queries';
	import { views, entities } from '$lib/shared/stores/metadata';
	import { tallyContainerElements } from '$lib/features/topology/labels';
	import type { TopologyEditState } from '$lib/features/topology/state';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import SubnetUtilization from '$lib/features/subnets/components/SubnetUtilization.svelte';
	import { common_utilization } from '$lib/paraglide/messages';
	import InspectorSection from '../shared/InspectorSection.svelte';
	import { SvelteSet } from 'svelte/reactivity';

	type Entity = components['schemas']['EntityDiscriminants'];

	let {
		node,
		topology,
		editState
	}: {
		node: Node;
		topology: RenderableTopology;
		editState: TopologyEditState;
	} = $props();

	// A subnet container's utilization, beside the counts of what it holds. It is derived
	// server-side across the whole site, so it comes from the live subnet list rather than the
	// topology payload. Not in a read-only view: a shared link has no session to fetch with, and a
	// snapshot's past contents would be paired with today's counts.
	let isSubnet = $derived(topology.subnets.some((s) => s.id === node.id));
	const subnetsQuery = useSubnetsQuery(
		undefined,
		undefined,
		() => isSubnet && !editState.isReadonly
	);
	let listedSubnet = $derived(
		isSubnet && !editState.isReadonly
			? ((subnetsQuery.data ?? []).find((s) => s.id === node.id) ?? null)
			: null
	);

	let elementConfig = $derived(
		(
			views.getMetadata($activeView) as
				| {
						element_config?: {
							element_entities?: Array<{
								entity_type: Entity;
								inline_entities: Entity[];
							}>;
							collective_noun?: string;
						};
				  }
				| undefined
		)?.element_config ?? {}
	);

	// Union of all element entity types and their declared inline entity types,
	// preserving order: element entity types first, then any inline-only ones.
	let summaryEntities = $derived.by((): Entity[] => {
		const seen = new SvelteSet<Entity>();
		const result: Entity[] = [];
		for (const ee of elementConfig.element_entities ?? []) {
			if (!seen.has(ee.entity_type)) {
				seen.add(ee.entity_type);
				result.push(ee.entity_type);
			}
		}
		for (const ee of elementConfig.element_entities ?? []) {
			for (const inline of ee.inline_entities) {
				if (!seen.has(inline)) {
					seen.add(inline);
					result.push(inline);
				}
			}
		}
		return result;
	});

	let counts = $derived(
		tallyContainerElements(node.id, topology, getInfrastructureRuleIdForTopology(topology))
	);
	let total = $derived([...counts.values()].reduce((s, n) => s + n, 0));

	function titleCase(s: string): string {
		if (!s) return s;
		return s[0].toUpperCase() + s.slice(1);
	}
</script>

<InspectorSection id="ElementSummary" section="ElementSummary">
	<div class="card card-static space-y-1 text-sm">
		{#if elementConfig.collective_noun}
			<div class="border-border mb-1 flex justify-between border-b pb-1 font-medium">
				<span class="text-secondary">{titleCase(elementConfig.collective_noun)}s</span>
				<span class="text-primary">{total}</span>
			</div>
		{/if}
		{#each summaryEntities as entity (entity)}
			{@const label = entities.getMetadata(entity)?.entity_name_plural ?? entity}
			<div class="flex justify-between">
				<span class="text-tertiary {elementConfig.collective_noun ? 'pl-3' : ''}">{label}</span>
				<span class="text-primary">{counts.get(entity) ?? 0}</span>
			</div>
		{/each}
		{#if listedSubnet}
			<div class="border-border mt-1 space-y-1 border-t pt-2">
				<span class="text-secondary font-medium">{common_utilization()}</span>
				<SubnetUtilization subnet={listedSubnet} />
			</div>
		{/if}
	</div>
</InspectorSection>
