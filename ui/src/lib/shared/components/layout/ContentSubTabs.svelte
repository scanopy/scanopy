<script lang="ts">
	import type { IconComponent } from '$lib/shared/utils/types';
	import type { Component } from 'svelte';
	import PageTitle, { providePageTabs } from './PageTitle.svelte';

	export interface SubTab {
		id: string;
		label: string;
		/** Hover note on the tab row's (i) while this tab is active. */
		subtitle?: string;
		icon: IconComponent;
		component: Component;
	}

	let {
		tabs,
		activeTab = $bindable(),
		isReadOnly = false,
		notifications
	}: {
		tabs: SubTab[];
		activeTab: string;
		isReadOnly: boolean;
		notifications?: Record<string, string>;
	} = $props();

	let showsTabs = $derived(tabs.length > 1);
	providePageTabs(() => showsTabs);
	let active = $derived(tabs.find((tab) => tab.id === activeTab));
</script>

<div>
	<!-- The tabs take the page title's row, so the toolbar lines up with a single page's. Hidden
	     with one visible tab, so the group renders as a plain single-entity page. -->
	{#if showsTabs}
		<PageTitle
			tabs={tabs.map((tab) => ({
				id: tab.id,
				label: tab.label,
				notification: notifications?.[tab.id]
			}))}
			{activeTab}
			subtitle={active?.subtitle ?? null}
			onSelectTab={(id) => (activeTab = id)}
		/>
	{/if}

	<!--
		Sub-tab content. `relative` on the collapsed wrapper is required for the same
		reason as the tab wrappers in `routes/+page.svelte` — without a containing
		block, absolutely positioned descendants (`sr-only` spans) escape
		`overflow: hidden` and add scroll height to `main`.
	-->
	{#each tabs as tab (tab.id)}
		<div class={activeTab !== tab.id ? 'relative h-0 overflow-hidden' : ''}>
			<tab.component {isReadOnly} isActive={activeTab === tab.id} />
		</div>
	{/each}
</div>
