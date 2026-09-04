<script lang="ts">
	import { SvelteURL } from 'svelte/reactivity';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
	import EmailVerificationBanner from '$lib/shared/components/feedback/EmailVerificationBanner.svelte';
	import DemoBanner from '$lib/shared/components/feedback/DemoBanner.svelte';
	import TrialEndingBanner from '$lib/shared/components/feedback/TrialEndingBanner.svelte';
	import NoPaymentMethodBanner from '$lib/shared/components/feedback/NoPaymentMethodBanner.svelte';
	import TrialExpiryModal from '$lib/shared/components/feedback/TrialExpiryModal.svelte';
	import PostStripeWelcomeBanner from '$lib/shared/components/feedback/PostStripeWelcomeBanner.svelte';
	import Sidebar from '$lib/shared/components/layout/Sidebar.svelte';
	import { onDestroy, onMount } from 'svelte';
	import { discoverySSEManager } from '$lib/features/discovery/queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';

	import {
		topologySSEManager,
		selectedTopologyId,
		activeView
	} from '$lib/features/topology/queries';
	import { get } from 'svelte/store';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import BillingPlanModal from '$lib/features/billing/BillingPlanModal.svelte';
	import DaemonPromptModal from '$lib/features/daemons/components/DaemonPromptModal.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		useOrganizationQuery,
		useDaemonPromptResponseMutation
	} from '$lib/features/organizations/queries';
	import { isBillingPlanActive } from '$lib/features/organizations/types';
	import { reopenSettingsAfterBilling } from '$lib/features/billing/stores';
	import {
		modalState,
		openModal,
		closeModal,
		initModalFromUrl
	} from '$lib/shared/stores/modal-registry';
	import ContentSubTabs from '$lib/shared/components/layout/ContentSubTabs.svelte';
	import type { SubTab } from '$lib/shared/components/layout/ContentSubTabs.svelte';

	// Read hash immediately during script initialization, before onMount
	const initialHash = typeof window !== 'undefined' ? window.location.hash.substring(1) : '';

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let isAuthenticated = $derived(currentUserQuery.data != null);
	let isCheckingAuth = $derived(currentUserQuery.isPending);

	// TanStack Query for daemons - used to determine default tab
	// Only fetch when authenticated to avoid 401 errors during onboarding
	const daemonsQuery = useDaemonsQuery({ enabled: () => isAuthenticated });

	// Billing modal: show when billing is enabled but user has no active plan
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let organization = $derived(organizationQuery.data);
	let needsPlanSelection = $derived(
		billingEnabled && organization != null && !isBillingPlanActive(organization)
	);
	// Suppresses needsPlanSelection after plan selection, before the org query reactively updates.
	// Without this, closeModal() clears $modalState but needsPlanSelection keeps showBillingModal true.
	let planJustActivated = $state(false);
	let showBillingModal = $derived(
		billingEnabled &&
			((needsPlanSelection && !planJustActivated) || $modalState.name === 'billing-plan')
	);

	// Daemon prompt: driven by modal registry
	let showDaemonPrompt = $derived($modalState.name === 'daemon-prompt');
	const daemonPromptResponseMutation = useDaemonPromptResponseMutation();
	// Don't nag Viewers (they can't install daemons) and never re-show the prompt once
	// the user has responded to it (either CTA persists an onboarding milestone).
	let isViewer = $derived(currentUserQuery.data?.permissions === 'Viewer');
	let daemonPromptResponded = $derived(
		(organization?.onboarding?.includes('DaemonPromptDismissed') ?? false) ||
			(organization?.onboarding?.includes('DaemonPromptAccepted') ?? false)
	);

	let activeTab = $state(initialHash || 'home');
	let appInitialized = $state(false);
	let sidebarCollapsed = $state(false);
	let dataLoadingStarted = $state(false);
	let showSettings = $state(false);
	// Billing-blocking states force the Settings modal open on the Billing tab
	// and make it non-dismissible. Past-due users have to update payment; paused
	// users have to click Resume Now before they can navigate elsewhere. The
	// inline alerts in BillingTab carry the matching urgent copy.
	let isBillingBlocking = $derived(
		organization?.plan_status === 'past_due' || organization?.plan_status === 'paused'
	);
	let allTabs = $state<
		Array<{
			id: string;
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			component: any;
			isReadOnly: boolean;
			subTabIds?: string[];
			subTabDefs?: SubTab[];
			subTabNotifications?: Record<string, string>;
		}>
	>([]);

	// Update URL hash when activeTab changes
	// Skip the first run — on page load the topology stores haven't hydrated
	// from URL params yet, so writing them back would overwrite with defaults.
	let tabEffectInitialized = false;
	$effect(() => {
		if (typeof window !== 'undefined' && activeTab) {
			if (!tabEffectInitialized) {
				tabEffectInitialized = true;
				return;
			}
			const url = new SvelteURL(window.location.href);
			if (activeTab === 'topology') {
				// Set topology params when entering the topology tab
				const topoId = get(selectedTopologyId);
				const view = get(activeView);
				if (topoId) url.searchParams.set('topologyId', topoId);
				url.searchParams.set('view', view);
			} else {
				// Clear topology-specific URL params when leaving
				url.searchParams.delete('topologyId');
				url.searchParams.delete('view');
			}
			url.hash = activeTab;
			window.history.replaceState(window.history.state, '', url.toString());
		}
	});

	// Set initial tab based on daemons (only if no hash was specified in URL)
	// Suppress when billing modal is showing — user must pick a plan first
	let initialTabSet = $state(false);
	$effect(() => {
		if (!initialHash && !initialTabSet && daemonsQuery.isSuccess && !showBillingModal) {
			const wantsDaemonSetup =
				$modalState.name === 'create-daemon' || $modalState.name === 'daemon-prompt';
			activeTab = wantsDaemonSetup ? 'daemons' : 'home';
			initialTabSet = true;
		}
	});

	// Auto-open settings modal to billing tab when past_due or paused —
	// either state requires user action before they can resume normal use.
	$effect(() => {
		if (isBillingBlocking && appInitialized) {
			openModal('settings', { tab: 'billing' });
		}
	});

	// Auto-show daemon prompt for new orgs that haven't installed a daemon yet.
	// Centralizes logic that previously lived in each registration path.
	let daemonPromptShown = $state(false);
	$effect(() => {
		if (
			appInitialized &&
			!daemonPromptShown &&
			!showBillingModal &&
			$modalState.name === null &&
			!isViewer &&
			organization?.onboarding?.includes('OrgCreated') &&
			!organization?.onboarding?.includes('FirstDaemonRegistered') &&
			!daemonPromptResponded &&
			daemonsQuery.isSuccess &&
			daemonsQuery.data?.length === 0
		) {
			daemonPromptShown = true;
			openModal('daemon-prompt');
		}
	});

	// Function to handle browser navigation (back/forward)
	function handleHashChange() {
		if (typeof window !== 'undefined') {
			const hash = window.location.hash.substring(1);
			if (hash && hash !== activeTab) {
				activeTab = hash;
			}
		}
	}

	// Initialize app when authenticated
	// TanStack Query handles data fetching in components - no need for cascading loads
	async function initializeApp() {
		if (dataLoadingStarted) return;
		dataLoadingStarted = true;

		// Connect SSE managers for real-time updates
		topologySSEManager.connect();
		discoverySSEManager.connect();

		appInitialized = true;
		initModalFromUrl();

		// Block billing modal deep-link in non-cloud environments
		if (!billingEnabled && $modalState.name === 'billing-plan') {
			closeModal();
		}
	}

	// Reactive effect: initialize app when authenticated
	// The layout handles auth check via TanStack Query, so we just wait for it to complete
	$effect(() => {
		if (isAuthenticated && !isCheckingAuth && !dataLoadingStarted) {
			initializeApp();
		}
	});

	onMount(() => {
		// Listen for hash changes (browser back/forward)
		if (typeof window !== 'undefined') {
			window.addEventListener('hashchange', handleHashChange);
		}
	});

	onDestroy(() => {
		topologySSEManager.disconnect();
		discoverySSEManager.disconnect();

		if (typeof window !== 'undefined') {
			window.removeEventListener('hashchange', handleHashChange);
		}
	});
</script>

{#if appInitialized}
	<div class="flex h-screen">
		<!-- Sidebar -->
		<div class="flex-shrink-0">
			<Sidebar
				bind:activeTab
				bind:collapsed={sidebarCollapsed}
				bind:allTabs
				bind:showSettings
				settingsInitialTab={isBillingBlocking ? 'billing' : 'account'}
				settingsDismissible={!isBillingBlocking}
			/>
		</div>

		<!-- Main Content -->
		<!--
			min-w-0: a flex child defaults to min-width:auto, so it cannot shrink
			below its content. Without it a wide table stretches main, then the flex
			row, and the whole page scrolls sideways instead of the table alone.

			relative: this is the scroll container, but nothing here was positioned, so
			it was the containing block for nothing. Absolutely positioned descendants
			resolved against the initial containing block instead and kept their static
			position — for a 59-row host table, `sr-only` spans a thousand pixels below
			the fold. Out of main's overflow, they extended the *document*, so the whole
			page scrolled and main slid out of view: scroll down and the table was gone.
			`relative` makes main their containing block, so its own overflow contains
			them and only main scrolls.
		-->
		<main
			class="relative min-w-0 flex-1 overflow-auto transition-all duration-300"
			class:ml-16={sidebarCollapsed}
			class:ml-48={!sidebarCollapsed}
		>
			{#if currentUserQuery.data && !currentUserQuery.data.email_verified}
				<EmailVerificationBanner email={currentUserQuery.data.email} />
			{/if}
			<TrialEndingBanner />
			<NoPaymentMethodBanner />
			<PostStripeWelcomeBanner />
			{#if organization?.plan?.type === 'Demo'}
				<DemoBanner />
			{/if}
			<div class="p-4 [&_.sticky]:sticky [&_.sticky]:top-0">
				<!--
					Programmatically render all tabs based on sidebar config.

					`relative` on the collapsed wrappers is load-bearing, not decoration.
					`overflow: hidden` only clips descendants whose containing block is the
					clipper or inside it, and a static box is the containing block for
					nothing absolutely positioned. Tailwind's `sr-only` is `position:
					absolute`, so every empty-value span `FieldValue` renders escaped the
					clip, resolved against the nearest positioned ancestor, and kept its
					static position deep inside the hidden tab's table — leaving `main` with
					~2200px of scrollable nothing under a short table. `relative` makes the
					zero-height wrapper the containing block, so those spans are clipped
					with everything else.
				-->
				{#each allTabs as tab (tab.id)}
					{#if tab.subTabIds && tab.subTabDefs}
						<div class={!tab.subTabIds.includes(activeTab) ? 'relative h-0 overflow-hidden' : ''}>
							<ContentSubTabs
								tabs={tab.subTabDefs}
								bind:activeTab
								isReadOnly={tab.isReadOnly}
								notifications={tab.subTabNotifications}
							/>
						</div>
					{:else}
						<div class={activeTab !== tab.id ? 'relative h-0 overflow-hidden' : ''}>
							<tab.component isReadOnly={tab.isReadOnly} isActive={activeTab === tab.id} />
						</div>
					{/if}
				{/each}
			</div>

			<Toast />
		</main>
	</div>

	<TrialExpiryModal />

	<!-- Billing modal rendered last so it stacks on top of other modals -->
	<BillingPlanModal
		isOpen={showBillingModal}
		name="billing-plan"
		dismissible={!needsPlanSelection}
		onClose={() => {
			planJustActivated = true;
			closeModal();
			if ($reopenSettingsAfterBilling) {
				reopenSettingsAfterBilling.set(false);
				openModal('settings', { tab: 'billing' });
			} else if (!isViewer && !daemonPromptResponded && daemonsQuery.data?.length === 0) {
				// Mark as shown here too so the first Skip click sticks — otherwise the
				// auto-open $effect re-fires on close (its guard was never set on this path).
				daemonPromptShown = true;
				openModal('daemon-prompt');
			}
		}}
	/>

	<DaemonPromptModal
		isOpen={showDaemonPrompt}
		onInstall={() => {
			daemonPromptResponseMutation.mutate('accepted');
			openModal('create-daemon');
		}}
		onSkip={() => {
			daemonPromptResponseMutation.mutate('dismissed');
			closeModal();
		}}
	/>
{:else}
	<!-- Data still loading -->
	<Loading />
{/if}
