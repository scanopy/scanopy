<script lang="ts" module>
	export const COOKIE_NAME = 'scanopy_gdpr';

	export interface CookiePreferences {
		necessary: boolean;
		analytics: boolean;
		marketing: boolean;
	}

	export function getCookie(name: string): string | null {
		if (typeof document === 'undefined') return null;
		const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'));
		return match ? decodeURIComponent(match[2]) : null;
	}

	export function getGdprPreferences(): CookiePreferences | null {
		const saved = getCookie(COOKIE_NAME);
		if (!saved) return null;
		try {
			return JSON.parse(saved) as CookiePreferences;
		} catch {
			return null;
		}
	}

	export function hasAnalyticsConsent(): boolean {
		return getGdprPreferences()?.analytics ?? false;
	}

	export function hasMarketingConsent(): boolean {
		return getGdprPreferences()?.marketing ?? false;
	}
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import CookieSettingsPanel from './CookieSettingsPanel.svelte';
	import {
		common_close,
		common_customize,
		cookies_acceptAll,
		cookies_preferences,
		cookies_rejectAll,
		cookies_settings,
		cookies_settingsDesc
	} from '$lib/paraglide/messages';

	let showBanner = $state(false);
	let showSettings = $state(false);
	let mounted = $state(false);
	let hasConsented = $state(false);

	let bannerPanel = $state<CookieSettingsPanel>();

	onMount(() => {
		mounted = true;
		const saved = getGdprPreferences();
		if (saved) {
			hasConsented = true;
		} else {
			showBanner = true;
		}
	});

	function handlePanelSave() {
		hasConsented = true;
		showBanner = false;
		showSettings = false;
	}

	function openSettings() {
		showSettings = true;
		showBanner = true;
	}

	function closeSettings() {
		showSettings = false;
		if (hasConsented) {
			showBanner = false;
		}
	}
</script>

{#if mounted}
	{#if showBanner && !showSettings}
		<div class="spacer"></div>
	{/if}
	{#if showBanner}
		<div class="overlay" class:visible={showSettings}></div>
		<div class="banner" class:settings-open={showSettings}>
			{#if showSettings}
				<div class="settings-panel">
					<div class="settings-header">
						<h3 class="title">{cookies_preferences()}</h3>
						<button class="close-btn" onclick={closeSettings} aria-label={common_close()}>
							<svg
								xmlns="http://www.w3.org/2000/svg"
								width="20"
								height="20"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
								stroke-linecap="round"
								stroke-linejoin="round"
							>
								<line x1="18" y1="6" x2="6" y2="18"></line>
								<line x1="6" y1="6" x2="18" y2="18"></line>
							</svg>
						</button>
					</div>
					<CookieSettingsPanel onSave={handlePanelSave} />
				</div>
			{:else}
				<div class="content">
					<div class="text-content">
						<h3 class="title">{cookies_settings()}</h3>
						<p class="description">
							<!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted i18n content -->
							{@html cookies_settingsDesc()}
						</p>
					</div>
					<div class="buttons">
						<button class="btn btn-link" onclick={openSettings}>{common_customize()}</button>
						<button class="btn btn-secondary" onclick={() => bannerPanel?.rejectAll()}
							>{cookies_rejectAll()}</button
						>
						<button class="btn btn-primary" onclick={() => bannerPanel?.acceptAll()}
							>{cookies_acceptAll()}</button
						>
					</div>
				</div>
				<CookieSettingsPanel bind:this={bannerPanel} onSave={handlePanelSave} hidden={true} />
			{/if}
		</div>
	{/if}
{/if}

<style>
	.spacer {
		height: 100px;
	}

	@media (min-width: 768px) {
		.spacer {
			height: 72px;
		}
	}

	.overlay {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0);
		z-index: 9998;
		pointer-events: none;
		transition: background 200ms;
	}

	.overlay.visible {
		background: rgba(0, 0, 0, 0.5);
		pointer-events: auto;
	}

	.banner {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		background: var(--color-bg-elevated);
		border-top: 1px solid var(--color-border);
		padding: 1.25rem;
		z-index: 9999;
	}

	.banner.settings-open {
		bottom: auto;
		top: 50%;
		left: 50%;
		right: auto;
		transform: translate(-50%, -50%);
		max-width: 500px;
		width: calc(100% - 2rem);
		border: 1px solid var(--color-border);
		border-radius: 0.5rem;
		max-height: 90vh;
		overflow-y: auto;
	}

	.content {
		max-width: 1200px;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.text-content {
		flex: 1;
	}

	@media (min-width: 768px) {
		.content {
			flex-direction: row;
			align-items: center;
			justify-content: space-between;
		}
	}

	.title {
		color: var(--color-text-primary);
		font-size: 1rem;
		font-weight: 600;
		margin: 0 0 0.25rem 0;
	}

	.description {
		color: var(--color-text-secondary);
		font-size: 0.875rem;
		margin: 0;
	}

	/* :global() needed because links come from @html i18n content */
	.description :global(a) {
		color: #2563eb;
		text-decoration: underline;
	}

	:global(.dark) .description :global(a) {
		color: #60a5fa;
	}

	.description :global(a:hover) {
		color: #1d4ed8;
	}

	:global(.dark) .description :global(a:hover) {
		color: #93c5fd;
	}

	.buttons {
		display: flex;
		gap: 0.5rem;
		flex-shrink: 0;
		flex-wrap: wrap;
	}

	.btn {
		padding: 0.5rem 1rem;
		border-radius: 0.375rem;
		font-weight: 500;
		font-size: 0.875rem;
		cursor: pointer;
		transition:
			background-color 150ms,
			border-color 150ms,
			color 150ms;
	}

	.btn-primary {
		background: #1d4ed8;
		color: white;
		border: 1px solid #2563eb;
	}

	.btn-primary:hover {
		background: #2563eb;
		border-color: #3b82f6;
	}

	.btn-secondary {
		background: transparent;
		color: var(--color-text-secondary);
		border: 1px solid var(--color-border);
	}

	.btn-secondary:hover {
		background: var(--color-bg-surface-hover);
		border-color: var(--color-border-input);
		color: var(--color-text-primary);
	}

	.btn-link {
		background: transparent;
		color: var(--color-text-secondary);
		border: 1px solid transparent;
		text-decoration: underline;
	}

	.btn-link:hover {
		color: var(--color-text-primary);
	}

	/* Settings panel styles */
	.settings-panel {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.settings-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.close-btn {
		background: transparent;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
		padding: 0.25rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 0.25rem;
	}

	.close-btn:hover {
		color: var(--color-text-primary);
		background: var(--color-bg-surface-hover);
	}
</style>
