<script lang="ts">
	import { pushSuccess, pushWarning } from '#lib/shared/stores/feedback.ts';
	import { copyText } from '#lib/shared/utils/clipboard.ts';
	import { Braces, X } from '@lucide/svelte';
	import Prism from '@magidoc/plugin-svelte-prismjs';
	import 'prismjs/components/prism-yaml';
	import 'prismjs/components/prism-json';
	import 'prismjs/components/prism-bash';
	import 'prismjs/components/prism-powershell';
	import 'prismjs/themes/prism-twilight.css';
	import {
		common_collapse,
		common_copied,
		common_copy,
		common_failedToCopy
	} from '#lib/paraglide/messages.js';
	import { useConfigQuery, isCloud } from '#lib/shared/stores/config-query.ts';

	let {
		code,
		expandable = true,
		expanded = $bindable(true),
		language = 'json',
		maxHeight = 'max-h-80',
		onCopy,
		hideCopyButton = false,
		preventSelect = false
	}: {
		code: string;
		expandable?: boolean;
		expanded?: boolean;
		language?: string;
		maxHeight?: string;
		onCopy?: () => void;
		hideCopyButton?: boolean;
		preventSelect?: boolean;
	} = $props();

	const configQuery = useConfigQuery();

	const isLocalhost =
		window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';

	const isSecureContext =
		window.isSecureContext ||
		window.location.hostname === 'localhost' ||
		window.location.hostname === '127.0.0.1';

	let shouldPreventSelect = $derived(
		preventSelect && !isLocalhost && configQuery.data && isCloud(configQuery.data)
	);

	async function copyJson() {
		try {
			await copyText(code);
			if (onCopy) {
				onCopy();
			} else {
				pushSuccess(common_copied());
			}
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}

	function toggleJson() {
		expanded = !expanded;
	}
</script>

<div>
	{#if expandable}
		<div class={`flex items-center justify-between  ${expanded ? 'mb-1' : ''}`}>
			<button
				type="button"
				class="flex items-center gap-1 p-1 text-xs text-tertiary transition-colors hover:text-secondary"
				onclick={toggleJson}
			>
				<Braces class="h-3 w-3" />
				<span>JSON</span>
			</button>
			{#if expanded}
				<button
					type="button"
					class="p-1 text-tertiary transition-colors hover:text-secondary"
					onclick={toggleJson}
					title={common_collapse()}
				>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>
	{/if}

	{#if expanded}
		<div translate="no" class="code-wrapper {maxHeight ? maxHeight + ' overflow-y-auto' : ''}">
			<div class="min-w-0 flex-1 {shouldPreventSelect ? 'prevent-select' : ''}">
				<Prism {language} showCopyButton={false} source={code} showLineNumbers={true} />
			</div>
			{#if isSecureContext && !hideCopyButton}
				<div class="copy-column">
					<button type="button" class="btn-icon" title={common_copy()} onclick={copyJson}>
						{common_copy()}
					</button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.code-wrapper {
		display: flex;
		align-items: stretch;
		background: hsl(0, 0%, 8%);
		border: 2px solid #6b7280;
		border-radius: 0.375rem;
		min-width: 0;
		max-width: 100%;
	}

	:global(.prism--code-container) {
		margin: 0 !important;
		border: none !important;
		background: transparent !important;
		max-width: 100% !important;
		overflow-x: hidden !important;
		border-radius: 0 !important;
	}

	:global(.prism--code-container pre),
	:global(.prism--code-container code) {
		white-space: pre-wrap !important;
		font-size: 0.75rem;
		word-wrap: break-word !important;
		overflow-wrap: break-word !important;
	}

	:global(.prism--code-container pre) {
		max-width: 100% !important;
		overflow-x: hidden !important;
		background: transparent !important;
	}

	@media (min-width: 640px) {
		:global(.prism--code-container pre),
		:global(.prism--code-container code) {
			font-size: 0.875rem;
		}
	}

	.copy-column {
		display: flex;
		align-items: flex-start;
		padding: 0.5rem;
	}

	.prevent-select :global(*) {
		user-select: none !important;
		-webkit-user-select: none !important;
	}
</style>
