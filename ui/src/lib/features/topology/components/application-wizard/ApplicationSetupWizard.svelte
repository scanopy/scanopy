<script lang="ts">
	import TopologyOverlay from '../TopologyOverlay.svelte';
	import DefineGroupsStep from './steps/DefineGroupsStep.svelte';
	import AssignEntitiesStep from './steps/AssignEntitiesStep.svelte';
	import type { Tag } from '#lib/features/tags/types/base.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		appWizard_title,
		appWizard_complete,
		common_back,
		common_next
	} from '#lib/paraglide/messages.js';

	let {
		appTags,
		siteId,
		onComplete
	}: {
		appTags: Tag[];
		siteId: string;
		onComplete: () => void;
	} = $props();

	let activeTab = $state<'define' | 'assign'>('define');

	let actions = $derived<ModalAction[]>(
		activeTab === 'define'
			? [
					{
						role: 'primary',
						label: common_next(),
						onClick: () => (activeTab = 'assign'),
						disabled: appTags.length === 0
					}
				]
			: [
					{
						role: 'secondary',
						label: common_back(),
						onClick: () => (activeTab = 'define'),
						align: 'start'
					},
					{ role: 'primary', label: appWizard_complete(), onClick: onComplete }
				]
	);
</script>

<TopologyOverlay title={appWizard_title()} size="xl" fixedHeight={true} {actions}>
	<div class="flex min-h-0 flex-1 flex-col p-6">
		{#if activeTab === 'define'}
			<div class="overflow-y-auto">
				<DefineGroupsStep {appTags} />
			</div>
		{:else if activeTab === 'assign'}
			<AssignEntitiesStep {appTags} {siteId} />
		{/if}
	</div>
</TopologyOverlay>
