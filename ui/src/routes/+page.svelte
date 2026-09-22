<script lang="ts">
	import { SvelteURL } from 'svelte/reactivity';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
	import AppBanners from '$lib/shared/components/feedback/AppBanners.svelte';
	import TrialExpiryModal from '$lib/shared/components/feedback/TrialExpiryModal.svelte';
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
	import { useConfigQuery, isLicenseSigningAvailable } from '$lib/shared/stores/config-query';
	import {
		useOrganizationQuery,
		useDaemonPromptResponseMutation
	} from '$lib/features/organizations/queries';
	import {
		hasLicensedPlan,
		isBillingPlanActive,
		isPlanLapsed
	} from '$lib/features/organizations/types';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { licenseTabVisible } from '$lib/features/settings/license-tab';
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

	// Billing modal: show when billing is enabled but user has no active plan
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	let billingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
	let licenseSigningAvailable = $derived(
		configQuery.data != null && isLicenseSigningAvailable(configQuery.data)
	);
	let organization = $derived(organizationQuery.data);
	// A licensed self-hosted plan on a billing-enabled server locks the main app:
	// the backend rejects main-app routes, and the org only gets Settings (its keys
	// live on the License tab). Switching to a cloud plan unlocks it on the next
	// org refetch.
	// Set when the plan picker closes on a licensed plan and cleared once the org
	// confirms it. The org's plan only flips after the Stripe webhook, so without
	// this the picker closes onto the main app for a second or two before Settings
	// opens on the License tab. The cache is deliberately not seeded instead:
	// waitForOrgUpdate invalidates and refetches at once, so a seeded plan reverts
	// within milliseconds and the License tab flickers out of the tab list.
	let licensedPlanJustPicked = $state(false);
	let isSelfHostedPlanLocked = $derived(
		billingEnabled &&
			(licensedPlanJustPicked || (organization != null && hasLicensedPlan(organization)))
	);
	$effect(() => {
		if (licensedPlanJustPicked && organization != null && hasLicensedPlan(organization)) {
			licensedPlanJustPicked = false;
		}
	});
	// Main-app data (SSE streams, daemons) waits until the lock state is known.
	let mainAppAvailable = $derived(
		configQuery.data != null && organization != null && !isSelfHostedPlanLocked
	);

	// TanStack Query for daemons - used to determine default tab
	// Only fetch when authenticated to avoid 401 errors during onboarding,
	// and not while the main app is locked (the route would 403)
	const daemonsQuery = useDaemonsQuery({
		enabled: () => isAuthenticated && !isSelfHostedPlanLocked
	});
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
	let isOwner = $derived(currentUserQuery.data?.permissions === 'Owner');
	// A lapsed cloud org (subscription ended, no paid plan chosen since) keeps
	// its plan and browses read-only: the backend refuses writes, the banner
	// carries the CTA, and the plan picker opens once on load for owners so
	// the way back is in front of them without locking them out of the app.
	let isLapsed = $derived(billingEnabled && organization != null && isPlanLapsed(organization));
	let lapsedPickerShown = $state(false);
	$effect(() => {
		if (isLapsed && isOwner && appInitialized && !lapsedPickerShown && !$modalState.name) {
			lapsedPickerShown = true;
			openModal('billing-plan');
		}
	});
	let daemonPromptResponded = $derived(
		(organization?.onboarding?.includes('DaemonPromptDismissed') ?? false) ||
			(organization?.onboarding?.includes('DaemonPromptAccepted') ?? false)
	);

	let activeTab = $state(initialHash || 'home');
	let appInitialized = $state(false);
	let sidebarCollapsed = $state(false);
	let dataLoadingStarted = $state(false);
	let showSettings = $state(false);
	// Billing-blocking states force the Settings modal open and make it
	// non-dismissible. Past-due users have to update payment; paused users have to
	// click Resume Now before they can navigate elsewhere; orgs on a licensed
	// self-hosted plan manage their license and card on the License tab. The inline
	// alerts in BillingTab carry the matching urgent copy.
	let isBillingBlocking = $derived(
		(billingEnabled &&
			(organization?.plan_status === 'past_due' || organization?.plan_status === 'paused')) ||
			isSelfHostedPlanLocked
	);
	// Only owners can see the Billing tab; everyone else is held on Account,
	// where SettingsModal explains that an owner has to resolve billing. An owner on
	// a licensed self-hosted plan is held on License instead — the key is what that
	// org came for, and Billing has nothing it must act on.
	// This modal can't be dismissed, so the forced tab has to be one SettingsModal
	// shows. Both read licenseTabVisible with the same inputs: an owner whose License
	// tab is hidden (a server that lost its signing key, a demo org) goes to Billing.
	// A true result already implies isSelfHostedPlanLocked and isOwner.
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
	let billingBlockingTab = $derived(
		licenseTabVisible({
			isOwner,
			isDemoOrg,
			billingEnabled,
			signingAvailable: licenseSigningAvailable,
			hasLicensedPlan: organization != null && hasLicensedPlan(organization),
			licensedPlanPending: licensedPlanJustPicked
		})
			? 'license'
			: isOwner
				? 'billing'
				: 'account'
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

	// Auto-open settings modal to billing tab when past_due, paused, or on a
	// licensed self-hosted plan — each requires owner action before normal use.
	$effect(() => {
		if (isBillingBlocking && appInitialized) {
			openModal('settings', { tab: billingBlockingTab });
		}
	});

	// A URL deep link (initModalFromUrl) can name a main-app modal, and the org may
	// load after it opened. While locked, only the settings and billing modals stay.
	const LOCKED_ORG_MODALS = ['settings', 'billing-plan', 'payment-method', 'support'];
	$effect(() => {
		const name = $modalState.name;
		if (isSelfHostedPlanLocked && name && !LOCKED_ORG_MODALS.includes(name)) {
			closeModal();
		}
	});

	// Real-time streams are main-app routes: connect only once the org is known
	// to be unlocked, and drop them if the org moves onto a licensed plan.
	$effect(() => {
		if (!appInitialized) return;
		if (mainAppAvailable) {
			topologySSEManager.connect();
			discoverySSEManager.connect();
		} else {
			topologySSEManager.disconnect();
			discoverySSEManager.disconnect();
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
			!isSelfHostedPlanLocked &&
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

		// SSE managers connect from the mainAppAvailable effect once the org loads.
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
				settingsInitialTab={isBillingBlocking ? billingBlockingTab : 'account'}
				settingsDismissible={!isBillingBlocking}
				mainAppLocked={isSelfHostedPlanLocked}
				licensedPlanPending={licensedPlanJustPicked}
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
			<!-- Only while the main app is reachable. When it is gated, the Settings
			     modal carries the same stack in its frame instead, so exactly one copy
			     is ever mounted: AppBanner holds its dismissed state locally, and a
			     second copy behind the overlay would not follow a dismissal. -->
			{#if !isBillingBlocking}
				<AppBanners />
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
				<!-- Main-app tabs mount only once the org is known to be unlocked; their
				     queries hit routes a licensed self-hosted org is rejected from. -->
				{#if mainAppAvailable}
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
				{/if}
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
		onClose={(selectedPlan) => {
			planJustActivated = true;
			// Key the licensed check off the plan the user just picked: `organization`
			// still holds the previous plan at this point, which is how the daemon
			// prompt used to win the race and ask a self-hosted buyer to install a
			// daemon they can't reach. The lock effect opens Settings on the License
			// tab as soon as the org query catches up.
			const licensed =
				selectedPlan != null && billingPlans.getMetadata(selectedPlan.type).license_plan != null;
			closeModal();
			if (licensed) {
				// Locks the app and opens Settings on the License tab now, rather than
				// when the webhook lands the plan on the org.
				licensedPlanJustPicked = true;
				daemonPromptShown = true;
				reopenSettingsAfterBilling.set(false);
			} else if ($reopenSettingsAfterBilling) {
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
