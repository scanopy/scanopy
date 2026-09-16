<script lang="ts">
	import TopologyOverlay from '../TopologyOverlay.svelte';
	import DefineGroupsStep from './steps/DefineGroupsStep.svelte';
	import AssignEntitiesStep from './steps/AssignEntitiesStep.svelte';
	import type { Tag } from '$lib/features/tags/types/base';
	import {
		appWizard_title,
		appWizard_complete,
		common_back,
		common_next
	} from '$lib/paraglide/messages';

	let {
		appTags,
		networkId,
		onComplete
	}: {
		appTags: Tag[];
		networkId: string;
		onComplete: () => void;
	} = $props();

	let activeTab = $state<'define' | 'assign'>('define');
</script>

<TopologyOverlay title={appWizard_title()} size="xl" fixedHeight={true}>
	<div class="flex min-h-0 flex-1 flex-col p-6">
		{#if activeTab === 'define'}
			<div class="overflow-y-auto">
				<DefineGroupsStep {appTags} />
			</div>
		{:else if activeTab === 'assign'}
			<AssignEntitiesStep {appTags} {networkId} />
		{/if}
	</div>

	{#snippet footer()}
		<div class="modal-footer flex items-center justify-between">
			<div>
				{#if activeTab === 'assign'}
					<button type="button" class="btn-secondary" onclick={() => (activeTab = 'define')}>
						{common_back()}
					</button>
				{/if}
			</div>
			<div>
				{#if activeTab === 'define'}
					<button
						type="button"
						class="btn-primary"
						disabled={appTags.length === 0}
						onclick={() => (activeTab = 'assign')}
					>
						{common_next()}
					</button>
				{:else}
					<button type="button" class="btn-primary" onclick={onComplete}>
						{appWizard_complete()}
					</button>
				{/if}
			</div>
		</div>
	{/snippet}
</TopologyOverlay>
