<script lang="ts">
	import posthog from 'posthog-js';
	import { dev } from '$app/environment';
	import { type CookiePreferences, COOKIE_NAME, getGdprPreferences } from './CookieConsent.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import {
		common_analytics,
		common_marketing,
		common_necessary,
		cookies_acceptAll,
		cookies_alwaysOn,
		cookies_analyticsDesc,
		cookies_marketingDesc,
		cookies_necessaryDesc,
		cookies_preferencesDesc,
		cookies_rejectAll,
		cookies_savePreferences
	} from '$lib/paraglide/messages';

	let { onSave, hidden = false }: { onSave?: () => void; hidden?: boolean } = $props();

	const COOKIE_DOMAIN = dev
		? ''
		: typeof window !== 'undefined' && window.location.hostname.endsWith('.scanopy.net')
			? '.scanopy.net'
			: '';
	const COOKIE_DAYS = 365;

	let preferences: CookiePreferences = $state({
		necessary: true,
		analytics: false,
		marketing: false,
		...getGdprPreferences()
	});

	function setCookie(name: string, value: string, days: number) {
		const expires = new Date(Date.now() + days * 864e5).toUTCString();
		const domain = COOKIE_DOMAIN ? `; domain=${COOKIE_DOMAIN}` : '';
		document.cookie = `${name}=${encodeURIComponent(value)}; expires=${expires}; path=/${domain}; SameSite=Lax`;
	}

	function applyPreferences() {
		if (posthog.__loaded) {
			if (preferences.analytics) {
				posthog.opt_in_capturing();
			} else {
				posthog.opt_out_capturing();
			}
		}
	}

	function savePreferences() {
		setCookie(COOKIE_NAME, JSON.stringify(preferences), COOKIE_DAYS);
		applyPreferences();
		onSave?.();
	}

	export function acceptAll() {
		preferences = { necessary: true, analytics: true, marketing: true };
		savePreferences();
	}

	export function rejectAll() {
		preferences = { necessary: true, analytics: false, marketing: false };
		savePreferences();
	}
</script>

{#if hidden}
	<!-- Hidden instance used only for acceptAll/rejectAll methods -->
{:else}
	<div class="space-y-4">
		<p class="text-secondary text-sm">
			<!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted i18n content -->
			{@html cookies_preferencesDesc()}
		</p>

		<div class="space-y-3">
			<!-- Necessary -->
			<InfoCard variant="compact">
				<div class="flex items-center justify-between">
					<label class="flex cursor-pointer items-center gap-3">
						<input type="checkbox" checked disabled class="accent-blue-600" />
						<span class="text-primary text-sm font-medium">{common_necessary()}</span>
					</label>
					<span class="text-tertiary text-xs uppercase tracking-wider">{cookies_alwaysOn()}</span>
				</div>
				<p class="text-secondary text-xs leading-relaxed">{cookies_necessaryDesc()}</p>
			</InfoCard>

			<!-- Analytics -->
			<InfoCard variant="compact">
				<div class="flex items-center justify-between">
					<label class="flex cursor-pointer items-center gap-3">
						<input type="checkbox" bind:checked={preferences.analytics} class="accent-blue-600" />
						<span class="text-primary text-sm font-medium">{common_analytics()}</span>
					</label>
				</div>
				<p class="text-secondary text-xs leading-relaxed">{cookies_analyticsDesc()}</p>
			</InfoCard>

			<!-- Marketing -->
			<InfoCard variant="compact">
				<div class="flex items-center justify-between">
					<label class="flex cursor-pointer items-center gap-3">
						<input type="checkbox" bind:checked={preferences.marketing} class="accent-blue-600" />
						<span class="text-primary text-sm font-medium">{common_marketing()}</span>
					</label>
				</div>
				<p class="text-secondary text-xs leading-relaxed">{cookies_marketingDesc()}</p>
			</InfoCard>
		</div>

		<div class="flex flex-wrap justify-end gap-2 border-t border-[var(--color-border)] pt-3">
			<button type="button" onclick={rejectAll} class="btn-secondary">{cookies_rejectAll()}</button>
			<button type="button" onclick={acceptAll} class="btn-secondary">{cookies_acceptAll()}</button>
			<button type="button" onclick={savePreferences} class="btn-primary">
				{cookies_savePreferences()}
			</button>
		</div>
	</div>
{/if}
