<script lang="ts">
	import {
		optionsPanelExpanded,
		selectedNode,
		selectedEdge,
		selectedNodes,
		previewEdges,
		topologyOptions,
		editingDependencyId,
		OPTIONS_PANEL_WIDTH_PX
	} from '../../queries';
	import type { RenderableTopology } from '../../types/base';
	import { get } from 'svelte/store';
	import { ChevronLeft, ChevronRight, Filter, Group, Eye } from 'lucide-svelte';
	import OptionsContent from './options/OptionsContent.svelte';
	import InspectorNode from './inspectors/InspectorNode.svelte';
	import InspectorEdge from './inspectors/InspectorEdge.svelte';
	import InspectorMultiSelect from './inspectors/InspectorMultiSelect.svelte';
	import {
		topology_collapsePanel,
		topology_expandPanel,
		common_filters,
		common_groupsLabel,
		common_display,
		topology_tutorialPanelHint
	} from '$lib/paraglide/messages';

	type OptionsTab = 'filter' | 'group' | 'visual';
	let activeTab = $state<OptionsTab>('filter');

	const tabs: { id: OptionsTab; label: string; icon: typeof Filter }[] = [
		{ id: 'filter', label: common_filters(), icon: Filter },
		{ id: 'group', label: common_groupsLabel(), icon: Group },
		{ id: 'visual', label: common_display(), icon: Eye }
	];

	let {
		topology,
		tutorialTopology,
		isReadOnly = false,
		raised = false,
		onClearSelection,
		onGroupCreated,
		onDependencyTypeChange
	}: {
		topology: RenderableTopology | undefined;
		tutorialTopology?: RenderableTopology;
		isReadOnly?: boolean;
		/**
		 * Lift the panel above a `TopologyOverlay`'s shroud, which otherwise covers it and
		 * swallows every click. Set it whenever the panel is what the user needs to act on while
		 * an overlay is up — the empty states, whose filters live in here.
		 */
		raised?: boolean;
		onClearSelection?: () => void;
		onGroupCreated?: (groupId: string) => void;
		onDependencyTypeChange?: (type: string) => void;
	} = $props();

	let isTutorial = $derived(!!tutorialTopology);
	let effectiveTopology = $derived(tutorialTopology ?? topology);

	let multiSelectedNodes = $state(get(selectedNodes));
	selectedNodes.subscribe((value) => {
		multiSelectedNodes = value;
	});

	// Auto-expand panel when something is selected
	$effect(() => {
		if ($selectedNode || $selectedEdge || multiSelectedNodes.length >= 2) {
			optionsPanelExpanded.set(true);
		}
	});

	// Clear preview edges when multi-selection drops below 2
	$effect(() => {
		if (multiSelectedNodes.length < 2) {
			previewEdges.set([]);
		}
	});

	let showingOptions = $derived(!$selectedNode && !$selectedEdge && multiSelectedNodes.length < 2);
</script>

<!-- Floating Panel -->
<div
	class="topology-options absolute left-4 top-4 duration-300 {isTutorial || raised
		? 'z-30'
		: 'z-10'} {$optionsPanelExpanded ? '' : 'w-auto'}"
	style={$optionsPanelExpanded ? `width: ${OPTIONS_PANEL_WIDTH_PX}px` : ''}
>
	<div class="card card-static p-0 shadow-lg">
		{#if $optionsPanelExpanded}
			<!-- Header with collapse button and tabs -->
			<div class="flex items-end">
				{#if !isTutorial}
					<button
						class="btn-icon flex-shrink-0 rounded-xl p-3"
						onclick={() => optionsPanelExpanded.set(false)}
						aria-label={topology_collapsePanel()}
					>
						<ChevronLeft class="text-secondary h-5 w-5" />
					</button>
				{/if}
				{#if showingOptions && !isTutorial}
					<div class="flex flex-1">
						{#each tabs as tab (tab.id)}
							<button
								class="flex flex-1 items-center justify-center gap-1.5 border-b-2 pb-2 pt-2.5 text-xs font-medium transition-colors {activeTab ===
								tab.id
									? 'border-blue-500 text-blue-400'
									: 'text-secondary hover:text-primary border-gray-300 dark:border-gray-700'}"
								onclick={() => (activeTab = tab.id)}
							>
								<tab.icon class="h-3.5 w-3.5" />
								{tab.label}
							</button>
						{/each}
					</div>
				{:else if !isTutorial}
					<div class="flex-1 border-b-2" style="border-color: var(--color-border)"></div>
				{/if}
			</div>

			<!-- Content area -->
			<div
				class="overflow-y-auto p-3"
				style="max-height: calc(100vh - {isTutorial || !$topologyOptions.local.show_minimap
					? 180
					: 350}px);"
			>
				{#if multiSelectedNodes.length >= 2 || $editingDependencyId}
					{@const editingDep = $editingDependencyId
						? (effectiveTopology?.dependencies.find((d) => d.id === $editingDependencyId) ?? null)
						: null}
					<InspectorMultiSelect
						topology={effectiveTopology}
						{isReadOnly}
						{isTutorial}
						onClearSelection={onClearSelection ?? (() => selectedNodes.set([]))}
						{onGroupCreated}
						{onDependencyTypeChange}
						editingDependency={editingDep}
						onDone={() => {
							editingDependencyId.set(null);
							selectedNodes.set([]);
						}}
					/>
				{:else if isTutorial}
					<div class="text-secondary py-4 text-center text-sm">
						{topology_tutorialPanelHint()}
					</div>
				{:else if $selectedNode}
					{#key $selectedNode.id}
						<InspectorNode node={$selectedNode} />
					{/key}
				{:else if $selectedEdge}
					{#key $selectedEdge.id}
						<InspectorEdge edge={$selectedEdge} />
					{/key}
				{:else}
					<OptionsContent {activeTab} renderableTopology={effectiveTopology} />
				{/if}
			</div>
		{:else if !isTutorial}
			<!-- Collapsed toggle button - just the chevron -->
			<button
				class="btn-icon rounded-2xl p-3"
				onclick={() => optionsPanelExpanded.set(true)}
				aria-label={topology_expandPanel()}
			>
				<ChevronRight class="text-secondary h-5 w-5" />
			</button>
		{/if}
	</div>
</div>
