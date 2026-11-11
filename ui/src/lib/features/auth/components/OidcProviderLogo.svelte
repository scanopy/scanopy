<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { OidcProviderMetadata } from '#lib/shared/stores/config-query.ts';

	let {
		provider,
		fallback
	}: {
		provider: OidcProviderMetadata;
		/** Drawn when the provider has no logo or its logo fails to load. */
		fallback?: Snippet;
	} = $props();

	// The logo that failed, so a provider whose logo changes gets another try.
	let failedLogo = $state<string | null>(null);
	let logo = $derived(provider.logo && provider.logo !== failedLogo ? provider.logo : null);
</script>

{#if logo}
	<img src={logo} alt={provider.name} class="h-5 w-5" onerror={() => (failedLogo = logo)} />
{:else}
	{@render fallback?.()}
{/if}
