<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import {
		getPublicShareMetadata,
		getPublicShareTopology,
		verifySharePassword,
		getStoredShareAccessToken,
		storeShareAccessToken,
		clearStoredShareAccessToken
	} from '../queries';
	import type { PublicShareMetadata, ShareWithTopology } from '../types/base';
	import type { ErrorCode } from '$lib/generated/error-codes';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import PasswordGate from './PasswordGate.svelte';
	import ReadOnlyTopologyViewer from './ReadOnlyTopologyViewer.svelte';
	import { AlertTriangle } from 'lucide-svelte';
	import { entityBundleFrom, toRenderableTopology } from '$lib/features/topology/enriched';
	import type { TopologyView } from '$lib/features/topology/queries';

	const SHARE_TOKEN_INVALID: ErrorCode = 'share_token_invalid';
	interface Props {
		shareId: string | undefined;
		isEmbed?: boolean;
	}

	let { shareId, isEmbed = false }: Props = $props();

	let shareMetadata: PublicShareMetadata | null = $state(null);
	let topologyData: ShareWithTopology | null = $state(null);
	let loading = $state(true);

	// Compose the slim topology row + the TopologyData bundle into a
	// RenderableTopology for the requested view — the same merge the app uses.
	let renderableTopology = $derived.by(() => {
		if (!topologyData) return null;
		const d = topologyData.data;
		return toRenderableTopology(
			topologyData.topology,
			entityBundleFrom(d),
			topologyData.share.name,
			currentView
		);
	});
	let viewLoading = $state(false);
	let error: string | null = $state(null);
	let passwordVerified = $state(false);
	let enabledViews: TopologyView[] = $state([]);
	let currentView: TopologyView = $state('L3Logical');

	// Apply theme override from query parameter (already handled by app.html flash script,
	// but we also lock it so the theme store doesn't override it during the session)
	let themeOverride: string | null = null;
	onMount(async () => {
		const params = new URLSearchParams(window.location.search);
		const t = params.get('theme');
		if (t === 'light' || t === 'dark') {
			themeOverride = t;
			document.documentElement.classList.toggle('dark', t === 'dark');
			document.documentElement.style.colorScheme = t;
		}
		await loadShare();
	});

	onDestroy(() => {
		// Restore user's theme preference when navigating away
		if (themeOverride && typeof window !== 'undefined') {
			const stored = localStorage.getItem('scanopy-theme') || 'system';
			const isDark =
				stored === 'dark' ||
				(stored === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
			document.documentElement.classList.toggle('dark', isDark);
			document.documentElement.style.colorScheme = isDark ? 'dark' : 'light';
		}
	});

	async function loadShare() {
		if (!shareId) {
			error = isEmbed ? 'Embed not found' : 'Share not found';
			loading = false;
			return;
		}

		loading = true;
		error = null;

		const metaResult = await getPublicShareMetadata(shareId);

		if (!metaResult.success || !metaResult.data) {
			error = metaResult.error || (isEmbed ? 'Embed not found' : 'Share not found');
			loading = false;
			return;
		}

		shareMetadata = metaResult.data;
		enabledViews = shareMetadata.enabled_views;
		currentView = enabledViews.includes('L3Logical')
			? 'L3Logical'
			: (enabledViews[0] ?? 'L3Logical');

		if (!shareMetadata.requires_password) {
			const topoResult = await getPublicShareTopology(shareId, {
				embed: isEmbed,
				view: currentView
			});

			if (!topoResult.success || !topoResult.data) {
				error = topoResult.error || 'Failed to load topology';
				loading = false;
				return;
			}

			topologyData = topoResult.data;
		} else {
			const storedToken = getStoredShareAccessToken(shareId);
			if (storedToken) {
				const result = await getPublicShareTopology(shareId, {
					embed: isEmbed,
					access_token: storedToken,
					view: currentView
				});
				if (result.success && result.data) {
					topologyData = result.data;
				} else if (result.code === SHARE_TOKEN_INVALID) {
					// Token is expired/revoked (e.g. password changed server-side).
					// Drop it and fall through to PasswordGate.
					clearStoredShareAccessToken(shareId);
				}
			}
		}

		loading = false;
	}

	async function handlePasswordSubmit(password: string): Promise<boolean> {
		if (!shareId) return false;

		const verifyResult = await verifySharePassword(shareId, password);
		if (!verifyResult.success || !verifyResult.access_token) {
			return false;
		}

		// Password accepted — persist the server-issued access token (never
		// the raw password) and close the gate.
		storeShareAccessToken(shareId, verifyResult.access_token);
		passwordVerified = true;

		// Now try to load topology - errors will show in main view, not gate
		const topoResult = await getPublicShareTopology(shareId, {
			embed: isEmbed,
			access_token: verifyResult.access_token,
			view: currentView
		});
		if (topoResult.success && topoResult.data) {
			topologyData = topoResult.data;
		} else {
			error = topoResult.error || 'Failed to load topology';
		}

		return true;
	}

	async function handleViewChange(view: string) {
		// The switcher hands back a plain string. Match it against the share's own
		// enabled views, which are typed: that narrows without re-listing the
		// union, and drops anything the share does not actually offer.
		const next = enabledViews.find((enabled) => enabled === view);
		if (!shareId || !next || next === currentView) return;

		viewLoading = true;
		currentView = next;

		const accessToken = shareMetadata?.requires_password
			? getStoredShareAccessToken(shareId)
			: null;

		const topoResult = await getPublicShareTopology(shareId, {
			embed: isEmbed,
			access_token: accessToken ?? undefined,
			view: next
		});

		if (topoResult.success && topoResult.data) {
			topologyData = topoResult.data;
		} else if (topoResult.code === SHARE_TOKEN_INVALID) {
			// Stored token expired or was revoked (password change).
			// Clear it and re-open PasswordGate for a fresh verification.
			clearStoredShareAccessToken(shareId);
			topologyData = null;
			passwordVerified = false;
		} else {
			error = topoResult.error || 'Failed to load topology';
		}

		viewLoading = false;
	}

	function getTitle(): string {
		if (topologyData?.share.name) return topologyData.share.name;
		if (shareMetadata?.name) return shareMetadata.name;
		return isEmbed ? 'Embedded Topology' : 'Shared Topology';
	}
</script>

<svelte:head>
	<title>{getTitle()} | Scanopy</title>
	{#if isEmbed}
		<style>
			body {
				margin: 0;
				padding: 0;
				overflow: hidden;
			}
		</style>
	{/if}
</svelte:head>

<div class="{isEmbed ? 'h-screen w-screen' : 'min-h-screen'} bg-[var(--color-bg-elevated)]">
	{#if loading}
		<div class="flex {isEmbed ? 'h-full' : 'min-h-screen'} items-center justify-center">
			<Loading />
		</div>
	{:else if error}
		<div
			class="flex {isEmbed
				? 'h-full'
				: 'min-h-screen'} flex-col items-center justify-center gap-2 p-4 text-center"
		>
			<AlertTriangle class="h-8 w-8 text-yellow-500" />
			<p class="text-secondary text-sm">{error}</p>
		</div>
	{:else if topologyData && renderableTopology}
		<div class={isEmbed ? 'h-full' : 'h-screen'}>
			<ReadOnlyTopologyViewer
				{shareId}
				topology={renderableTopology}
				shareName={isEmbed ? undefined : topologyData.share.name}
				showControls={topologyData.share.options.show_zoom_controls}
				showInspectPanel={topologyData.share.options.show_inspect_panel}
				showExport={!isEmbed && (topologyData.share.options.show_export_button ?? true)}
				showMinimap={topologyData.share.options.show_minimap ?? true}
				exportFeatures={topologyData.export_features}
				{isEmbed}
				{enabledViews}
				{currentView}
				onViewChange={handleViewChange}
				{viewLoading}
			/>
		</div>
	{/if}

	<PasswordGate
		isOpen={!!shareMetadata?.requires_password && !topologyData && !passwordVerified && !loading}
		title={shareMetadata?.name || 'Password Required'}
		onSubmit={handlePasswordSubmit}
	/>
</div>
