<script lang="ts">
	import { Loader2 } from '@lucide/svelte';
	import type { OidcProviderMetadata } from '#lib/shared/stores/config-query.ts';
	import OidcProviderLogo from './OidcProviderLogo.svelte';
	import {
		auth_signInWith,
		auth_signInWithEmail,
		common_lastUsed,
		common_or
	} from '#lib/paraglide/messages.js';

	interface Props {
		providers: OidcProviderMetadata[];
		lastLoginMethod?: string | null;
		disablePasswordLogin?: boolean;
		oidcLoadingSlug?: string | null;
		disabled?: boolean;
		onOidcSelect: (slug: string) => void;
		onEmailSelect: () => void;
		oidcButtonLabel?: (providerName: string) => string;
		emailButtonLabel?: string;
	}

	let {
		providers,
		lastLoginMethod = null,
		disablePasswordLogin = false,
		oidcLoadingSlug = null,
		disabled = false,
		onOidcSelect,
		onEmailSelect,
		oidcButtonLabel,
		emailButtonLabel
	}: Props = $props();

	function getOidcLabel(providerName: string): string {
		return oidcButtonLabel
			? oidcButtonLabel(providerName)
			: auth_signInWith({ provider: providerName });
	}

	let resolvedEmailLabel = $derived(emailButtonLabel ?? auth_signInWithEmail());
</script>

<div class="space-y-4">
	{#if providers.length > 0}
		<div class="space-y-2">
			{#each providers as provider (provider.slug)}
				<button
					type="button"
					onclick={() => onOidcSelect(provider.slug)}
					{disabled}
					class="btn-primary flex w-full items-center justify-center gap-3"
					class:opacity-50={oidcLoadingSlug !== null && oidcLoadingSlug !== provider.slug}
				>
					{#if oidcLoadingSlug === provider.slug}
						<Loader2 class="h-5 w-5 animate-spin" />
					{:else}
						<OidcProviderLogo {provider} />
					{/if}
					<span>{getOidcLabel(provider.name)}</span>
					{#if lastLoginMethod === `oidc:${provider.slug}`}
						<span class="rounded-full bg-white/20 px-2 py-0.5 text-xs font-medium">
							{common_lastUsed()}
						</span>
					{/if}
				</button>
			{/each}
		</div>

		{#if !disablePasswordLogin}
			<div class="relative">
				<div class="absolute inset-0 flex items-center">
					<div class="w-full border-t" style="border-color: var(--color-border)"></div>
				</div>
				<div class="relative flex justify-center text-sm">
					<span class="bg-(--color-bg-elevated) px-2 text-tertiary">{common_or()}</span>
				</div>
			</div>
		{/if}
	{/if}

	{#if !disablePasswordLogin}
		<button type="button" onclick={onEmailSelect} {disabled} class="btn-primary w-full">
			<span>{resolvedEmailLabel}</span>
			{#if lastLoginMethod === 'email'}
				<span class="ml-2 rounded-full bg-white/20 px-2 py-0.5 text-xs font-medium">
					{common_lastUsed()}
				</span>
			{/if}
		</button>
	{/if}
</div>
