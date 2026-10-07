<!--
	A page's title row, the same on every page. It is the height of the sidebar's logo row and sits
	8px above what follows, as the logo row does the sidebar search, so a page's toolbar lines up
	with global search. A subtitle is a hover note on an (i), and `aside` is context after the
	title, set in the title's style (Home's organization name); neither adds a second line.

	A page with sub-tabs (Daemons and API Keys) passes `tabs` instead of a title: the tabs take the
	title's place, styled as titles and underlined like modal tabs, so the toolbar below sits where it does on a single page. The
	sub-tab pages' own titles then render nothing, through `providePageTabs`.
-->
<script module lang="ts">
	import { getContext, setContext } from 'svelte';

	const PAGE_TABS_KEY = Symbol('page-tabs');

	/**
	 * Mark this subtree as under a tab row, so the pages in it leave the title to the tabs while
	 * `showsTabs` holds. A function, since the visible tabs change with permissions.
	 */
	export function providePageTabs(showsTabs: () => boolean) {
		setContext(PAGE_TABS_KEY, showsTabs);
	}

	export interface PageTitleTab {
		id: string;
		label: string;
		/** Color of a dot after the label, flagging something on that tab. */
		notification?: string;
	}
</script>

<script lang="ts">
	import { Info } from 'lucide-svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import { common_contentTabs } from '$lib/paraglide/messages';

	let {
		title = null,
		subtitle = null,
		aside = null,
		tabs = null,
		activeTab = null,
		onSelectTab = () => {}
	}: {
		title?: string | null;
		subtitle?: string | null;
		aside?: string | null;
		tabs?: PageTitleTab[] | null;
		activeTab?: string | null;
		onSelectTab?: (id: string) => void;
	} = $props();

	const showsTabs = getContext<(() => boolean) | undefined>(PAGE_TABS_KEY);
	let hidden = $derived(!tabs && (showsTabs?.() ?? false));
</script>

{#if !hidden}
	<div class="mb-2 flex h-[38px] min-w-0 items-center gap-2">
		{#if tabs}
			<div
				class="-ml-1 flex min-w-0 gap-5 self-stretch"
				role="tablist"
				aria-label={common_contentTabs()}
			>
				{#each tabs as tab (tab.id)}
					<button
						type="button"
						role="tab"
						aria-selected={tab.id === activeTab}
						class="relative flex shrink-0 items-center border-y-2 border-t-transparent px-1 text-xl font-bold transition-colors {tab.id ===
						activeTab
							? 'text-primary border-b-blue-500'
							: 'text-muted hover:text-secondary border-b-transparent'}"
						onclick={() => onSelectTab(tab.id)}
					>
						{tab.label}
						{#if tab.notification}
							<span
								class="absolute -right-1 top-2 h-2 w-2 rounded-full"
								style="background-color: {tab.notification}"
							></span>
						{/if}
					</button>
				{/each}
			</div>
		{:else}
			<h1 class="text-primary shrink-0 text-xl font-bold">{title}</h1>
		{/if}
		{#if subtitle}
			<span
				class="text-tertiary hover:text-secondary inline-flex cursor-help transition-colors"
				use:tooltip
				data-tooltip={subtitle}
				role="note"
				aria-label={subtitle}
			>
				<Info class="h-4 w-4" aria-hidden="true" />
			</span>
		{/if}
		{#if aside}
			<span aria-hidden="true" class="text-primary text-xl font-bold">·</span>
			<span class="text-primary truncate text-xl font-bold">{aside}</span>
		{/if}
	</div>
{/if}
