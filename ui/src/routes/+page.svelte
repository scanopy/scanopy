<script lang="ts">
	import Loading from '#lib/shared/components/feedback/Loading.svelte';
	import Toast from '#lib/shared/components/feedback/Toast.svelte';
	import AppBanners from '#lib/shared/components/feedback/AppBanners.svelte';
	import ServerUnreachableBanner from '#lib/shared/components/feedback/ServerUnreachableBanner.svelte';
	import TrialExpiryModal from '#lib/shared/components/feedback/TrialExpiryModal.svelte';
	import Sidebar from '#lib/shared/components/layout/Sidebar.svelte';
	import GlobalSearch from '#lib/shared/components/layout/GlobalSearch.svelte';
	import { onDestroy, onMount } from 'svelte';
	import { liveUpdatesSSEManager } from '#lib/shared/live-updates.ts';
	import { useCurrentUserQuery } from '#lib/features/auth/queries.ts';
	import { useCan } from '#lib/features/auth/access.ts';

	import { attachUrlState, setActiveTab } from '#lib/shared/stores/url-state.ts';
	import { useDaemonsQuery } from '#lib/features/daemons/queries.ts';
	import BillingPlanModal from '#lib/features/billing/BillingPlanModal.svelte';
	import CloudPlanRequired from '#lib/features/billing/CloudPlanRequired.svelte';
	import { waitForOrgUpdate } from '#lib/shared/billing/wait-for-org-update.ts';
	import { entityUIConfig, TAB_LABELS } from '#lib/shared/entity-ui-config.ts';
	import { useDashboardQuery } from '#lib/features/home/queries.ts';
	import { setupStepsLeft } from '#lib/features/home/setup-list.ts';
	import ReleaseNotesModal from '#lib/features/release-notes/ReleaseNotesModal.svelte';
	import TermsAcceptanceModal from '#lib/features/auth/components/TermsAcceptanceModal.svelte';
	import EmailVerificationGate from '#lib/features/auth/components/EmailVerificationGate.svelte';
	import AuthBackground from '#lib/shared/components/layout/AuthBackground.svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '#lib/api/query-client.ts';
	import {
		useUnseenReleaseNotesQuery,
		useMarkReleaseNotesSeenMutation
	} from '#lib/features/release-notes/queries.ts';
	import {
		useConfigQuery,
		isCloud,
		isLicenseSigningAvailable
	} from '#lib/shared/stores/config-query.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import {
		hasLicensedPlan,
		hasTopologyAccess,
		isBillingPlanActive,
		isPlanLapsed,
		isPlanUnchosen,
		isSelfHostedPlanLocked as isSelfHostedPlanLockedFor
	} from '#lib/features/organizations/types.ts';
	import {
		isShellReady,
		shouldCloseBillingPlanLink,
		shouldOpenLicenseSettings
	} from '#lib/features/organizations/onload-gates.ts';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { licenseTabVisible } from '#lib/features/settings/license-tab.ts';
	import {
		licenseSettingsOpenedFor as storedLicenseSettingsOpenedFor,
		markLicenseSettingsOpened
	} from '#lib/features/settings/license-settings-opened.ts';
	import { billingOverModal, reopenSettingsAfterBilling } from '#lib/features/billing/stores.ts';
	import {
		modalState,
		openModal,
		closeModal,
		initModalFromUrl
	} from '#lib/shared/stores/modal-registry.ts';
	import ContentSubTabs from '#lib/shared/components/layout/ContentSubTabs.svelte';
	import type { SubTab } from '#lib/shared/components/layout/ContentSubTabs.svelte';

	// Read hash immediately during script initialization, before onMount
	// `#home` names the tab Setup and Issues replaced, so a bookmark of it opens the landing tab.
	const RETIRED_HOME_HASH = 'home';
	const loadedHash = typeof window !== 'undefined' ? window.location.hash.substring(1) : '';
	const initialHash = loadedHash === RETIRED_HOME_HASH ? '' : loadedHash;
	// Before any tab mounts, so each hydrates from the params this page loaded with.
	const detachUrlState = typeof window !== 'undefined' ? attachUrlState() : null;

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	const can = useCan();
	let isAuthenticated = $derived(currentUserQuery.data != null);
	let isCheckingAuth = $derived(currentUserQuery.isPending);
	// The server refuses this user's main-app requests until they accept the
	// current terms, so the app stays unmounted behind the acceptance screen.
	let termsGated = $derived(currentUserQuery.data?.terms_acceptance_required === true);
	// The same for a user of an org on a license plan who hasn't verified their
	// email: the server decides, and the step replaces the app until they do.
	let verificationGated = $derived(currentUserQuery.data?.email_verification_required === true);

	// Billing modal: show when billing is enabled but user has no active plan
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let licenseSigningAvailable = $derived(
		configQuery.data != null && isLicenseSigningAvailable(configQuery.data)
	);
	let organization = $derived(organizationQuery.data);
	// The shell paints with the modal these two decide, rather than a request before it.
	let shellReady = $derived(isShellReady(organizationQuery, configQuery));
	// A licensed self-hosted plan on Scanopy Cloud locks the main app:
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
	// The server sets email_verification_required from the org's plan, so a plan
	// change (a license trial landing after sign-up, or a later switch) refetches
	// the current user to pick it up.
	const queryClient = useQueryClient();
	let lastPlanType: string | null | undefined = undefined;
	$effect(() => {
		const planType = organization?.plan?.type;
		if (planType === undefined) return;
		if (lastPlanType !== undefined && planType !== lastPlanType) {
			void queryClient.invalidateQueries({ queryKey: queryKeys.auth.currentUser() });
		}
		lastPlanType = planType;
	});
	let isSelfHostedPlanLocked = $derived(
		(isCloudDeployment && licensedPlanJustPicked) ||
			isSelfHostedPlanLockedFor(organization, isCloudDeployment)
	);
	$effect(() => {
		if (licensedPlanJustPicked && organization != null && hasLicensedPlan(organization)) {
			licensedPlanJustPicked = false;
		}
	});
	// A cloud sign-up that hasn't chosen a plan is locked to Settings the same
	// way: the backend opens only the Settings routes to it.
	let planUnchosen = $derived(isPlanUnchosen(organization, isCloudDeployment));
	// Set when the plan picker closes on a cloud plan. The plan only lands on the
	// org after the Stripe webhook, so this keeps Settings from being forced open
	// over the pending checkout. The main app still waits for the plan.
	let cloudPlanJustPicked = $state(false);
	let isMainAppLocked = $derived(isSelfHostedPlanLocked || planUnchosen);
	// Main-app data (SSE streams, daemons) waits until the lock state is known.
	let mainAppAvailable = $derived(
		configQuery.data != null &&
			organization != null &&
			!isMainAppLocked &&
			!termsGated &&
			!verificationGated
	);

	// TanStack Query for daemons - used to determine default tab
	// Only fetch when authenticated to avoid 401 errors during onboarding,
	// and only once the org is known not to be locked (the route would 403)
	const daemonsQuery = useDaemonsQuery({
		enabled: () => isAuthenticated && mainAppAvailable
	});
	let needsPlanSelection = $derived(
		isCloudDeployment && organization != null && !isBillingPlanActive(organization)
	);
	// Set whenever the picker closes, so it opens on its own at most once per load. It
	// also covers a plan selection before the org query reactively updates: without
	// it, closeModal() clears $modalState but needsPlanSelection keeps showBillingModal true.
	let planJustActivated = $state(false);

	let isOwner = $derived(can('Billing', 'Write'));

	// A license signup's trial starts off the signup itself, a second or two
	// after the org exists. A fresh org with no subscription yet waits for it
	// rather than flashing the plan picker; an older one is the fallback case and
	// gets the picker at once.
	const FRESH_SIGNUP_MS = 60_000;
	let licenseTrialWaitDone = $state(false);
	let licenseTrialPending = $derived(
		planUnchosen &&
			!licenseTrialWaitDone &&
			organization != null &&
			Date.now() - new Date(organization.created_at).getTime() < FRESH_SIGNUP_MS
	);
	let licenseTrialWaitStarted = $state(false);
	$effect(() => {
		if (!appInitialized || !licenseTrialPending || licenseTrialWaitStarted) return;
		licenseTrialWaitStarted = true;
		void waitForOrgUpdate((org) => org.plan_status != null, { intervalMs: 500 }).then(() => {
			licenseTrialWaitDone = true;
		});
	});
	let billingPlanOverModal = $derived($billingOverModal?.modal === 'billing-plan');
	let showBillingModal = $derived(
		isCloudDeployment &&
			((needsPlanSelection && !planJustActivated && !licenseTrialPending) ||
				$modalState.name === 'billing-plan')
	);

	// Empty until the landing tab is known, unless the URL named one.
	let activeTab = $state(initialHash);
	let appInitialized = $state(false);
	let sidebarCollapsed = $state(false);
	let dataLoadingStarted = $state(false);
	let showSettings = $state(false);
	// Billing-blocking states force the Settings modal open and make it
	// non-dismissible. Past-due users have to update payment; paused users have to
	// click Resume Now before they can navigate elsewhere; an org with no plan can
	// change its account and choose a plan from the Billing tab.
	// The inline alerts in BillingTab carry the matching urgent copy.
	let isBillingBlocking = $derived(
		(isCloudDeployment &&
			(organization?.plan_status === 'past_due' || organization?.plan_status === 'paused')) ||
			(planUnchosen && !cloudPlanJustPicked && !licenseTrialPending)
	);
	// Only owners can see the Billing tab; everyone else is held on Account,
	// where SettingsModal explains that an owner has to resolve billing.
	let billingBlockingTab = $derived(isOwner ? 'billing' : 'account');
	let isDemoOrg = $derived(
		billingPlans.getMetadata(organization?.plan?.type ?? null).is_demo === true
	);
	// An org on a licensed self-hosted plan gets Settings on the License tab when
	// the page loads, and can close it: the sidebar stays, and each page says it
	// needs a cloud plan. An owner whose License tab is hidden (a server
	// that lost its signing key, a demo org) gets Billing; everyone else Account.
	let licenseLockTab = $derived(
		licenseTabVisible({
			isOwner,
			isDemoOrg,
			isCloudDeployment,
			signingAvailable: licenseSigningAvailable
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
			subTabIds?: string[];
			subTabDefs?: SubTab[];
			subTabNotifications?: Record<string, string>;
		}>
	>([]);

	// The tab becomes the hash, and decides which params the URL carries: the new tab's list view or
	// topology, with the old tab's removed. Owners hydrate from the params the page loaded with, so
	// this runs on load too.
	$effect(() => {
		if (activeTab) setActiveTab(activeTab);
	});

	// The tab a load with no hash opens on: Setup while steps remain, else Issues while there are
	// any, else Topology. The sidebar shows the Setup and Issues entries on the same counts.
	const dashboardQuery = useDashboardQuery({
		enabled: () => isAuthenticated && mainAppAvailable
	});
	let landingTab = $derived.by(() => {
		const dashboard = dashboardQuery.data;
		if (!dashboard || !organization) return null;
		if (setupStepsLeft(dashboard.setup_items, isPlanLapsed(organization)) > 0) return 'setup';
		if (dashboard.issue_count > 0) return 'issues';
		return entityUIConfig.Topology!.tabId;
	});

	// Set the initial tab (only if no hash was specified in URL). Suppress while the plan picker
	// is showing.
	let initialTabSet = $state(false);
	$effect(() => {
		if (
			!initialHash &&
			!initialTabSet &&
			daemonsQuery.isSuccess &&
			landingTab &&
			!showBillingModal
		) {
			const wantsDaemonSetup = $modalState.name === 'create-daemon';
			activeTab = wantsDaemonSetup ? 'daemons' : landingTab;
			initialTabSet = true;
		}
	});

	// Auto-open settings modal to billing tab when past_due, paused, or with no
	// plan — each requires owner action before normal use. A sign-up whose plan
	// picker shows sees it first, and Settings opens once it closes. A plan picked
	// there may need a card first, and that dialog stays until it closes.
	// Nothing reopens while Settings is already up, so this never writes the
	// modal state it reads.
	$effect(() => {
		if (
			isBillingBlocking &&
			appInitialized &&
			!showBillingModal &&
			$modalState.name !== 'payment-method' &&
			$modalState.name !== 'settings'
		) {
			openModal('settings', { tab: billingBlockingTab });
		}
	});

	// An org on a licensed self-hosted plan gets Settings on the License tab once
	// per plan it lands on (on load, or after moving onto it by any path); closing
	// it leaves the sidebar, and a reload keeps it closed. Cleared when the org
	// leaves the plan.
	let licenseSettingsOpenedFor = $state<string | null>(null);
	$effect(() => {
		if (organization == null) return;
		// A license org whose trial never started is the billing gate's, above.
		const locked = isSelfHostedPlanLocked && !planUnchosen;
		if (!locked) {
			licenseSettingsOpenedFor = null;
			markLicenseSettingsOpened(organization.id, null);
		}
		const planType = organization.plan?.type ?? null;
		if (
			shouldOpenLicenseSettings({
				locked,
				planType,
				openedFor: licenseSettingsOpenedFor ?? storedLicenseSettingsOpenedFor(organization.id),
				appInitialized,
				planPickerShowing: showBillingModal,
				modalName: $modalState.name
			})
		) {
			licenseSettingsOpenedFor = planType;
			markLicenseSettingsOpened(organization.id, planType);
			openModal('settings', { tab: licenseLockTab });
		}
	});

	// A URL deep link (initModalFromUrl) can name a main-app modal, and the org may
	// load after it opened. While locked, only the settings and billing modals stay.
	const LOCKED_ORG_MODALS = ['settings', 'billing-plan', 'payment-method'];
	$effect(() => {
		const name = $modalState.name;
		if (isMainAppLocked && name && !LOCKED_ORG_MODALS.includes(name)) {
			closeModal();
		}
	});

	// The live-update stream is a main-app route: connect only once the org is known
	// to be unlocked, and drop it if the org moves onto a licensed plan. The server
	// reads topology access from the plan at connect, so a change to it reconnects.
	let streamTopologyAccess: boolean | null = null;
	$effect(() => {
		if (!appInitialized) return;
		if (!mainAppAvailable) {
			liveUpdatesSSEManager.disconnect();
			streamTopologyAccess = null;
			return;
		}
		const topologyAccess = hasTopologyAccess(organization);
		if (streamTopologyAccess !== null && streamTopologyAccess !== topologyAccess) {
			liveUpdatesSSEManager.disconnect();
		}
		streamTopologyAccess = topologyAccess;
		liveUpdatesSSEManager.connect();
	});

	// App-load modals show at most one per page load, in this order: the billing
	// gate (forced Settings), plan selection, trial expiry, the daemon prompt,
	// release notes. A lower one waits until everything above it has had its say.
	let trialModalOpen = $state(false);
	let higherModalShownThisLoad = $state(false);
	$effect(() => {
		if (isBillingBlocking || showBillingModal || trialModalOpen) {
			higherModalShownThisLoad = true;
		}
	});
	// Set when the URL named a modal at load; the user came for that one.
	let deepLinkOpened = $state(false);

	// Release notes: the releases since the user last dismissed them, opened
	// when one of them asks to be. Last in the order. Never in the demo.
	const unseenReleaseNotesQuery = useUnseenReleaseNotesQuery({
		enabled: () => isAuthenticated && mainAppAvailable && !isDemoOrg
	});
	const markReleaseNotesSeenMutation = useMarkReleaseNotesSeenMutation();
	let hasUnseenReleaseNotes = $derived((unseenReleaseNotesQuery.data?.releases.length ?? 0) > 0);
	let showReleaseNotes = $derived(
		$modalState.name === 'release-notes' && hasUnseenReleaseNotes && !isDemoOrg
	);
	let releaseNotesShown = $state(false);
	$effect(() => {
		if (
			appInitialized &&
			!releaseNotesShown &&
			!deepLinkOpened &&
			!higherModalShownThisLoad &&
			!isMainAppLocked &&
			$modalState.name === null &&
			hasUnseenReleaseNotes &&
			unseenReleaseNotesQuery.data?.auto_open
		) {
			releaseNotesShown = true;
			openModal('release-notes');
		}
	});
	// A `?modal=release-notes` link with nothing unseen, or in the demo, has nothing to show.
	$effect(() => {
		if (
			$modalState.name === 'release-notes' &&
			(isDemoOrg || (unseenReleaseNotesQuery.isSuccess && !hasUnseenReleaseNotes))
		) {
			closeModal();
		}
	});

	// Function to handle browser navigation (back/forward)
	function handleHashChange() {
		if (typeof window !== 'undefined') {
			const hash = window.location.hash.substring(1);
			if (hash === RETIRED_HOME_HASH) {
				if (landingTab) activeTab = landingTab;
			} else if (hash && hash !== activeTab) {
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
		deepLinkOpened = $modalState.name !== null;
	}

	// Block billing modal deep-link in non-cloud environments. Config can land after
	// initializeApp, so this waits for it rather than closing a Cloud link.
	$effect(() => {
		if (
			shouldCloseBillingPlanLink({
				modalName: $modalState.name,
				configLoaded: configQuery.data != null,
				isCloudDeployment
			})
		) {
			closeModal();
		}
	});

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
		liveUpdatesSSEManager.disconnect();

		if (typeof window !== 'undefined') {
			window.removeEventListener('hashchange', handleHashChange);
		}
		detachUrlState?.();
	});
</script>

{#if appInitialized && termsGated}
	<TermsAcceptanceModal isOpen={true} />
{:else if appInitialized && verificationGated}
	<div class="relative min-h-screen bg-(--color-bg-elevated)">
		<AuthBackground />
		<EmailVerificationGate isOpen={true} />
	</div>
{:else if appInitialized && shellReady}
	<div class="flex h-dvh flex-col">
		<!-- Banners span the full width, sidebar included.
		     Only while the main app is reachable. When it is gated, or the org is on
		     a licensed plan, the Settings modal carries the same stack in its frame
		     instead, so exactly one copy is ever mounted: AppBanner holds its
		     dismissed state locally, and a second copy behind the overlay would not
		     follow a dismissal. -->
		<div class="shrink-0">
			{#if !isBillingBlocking && !isSelfHostedPlanLocked}
				<AppBanners />
			{/if}
			<ServerUnreachableBanner />
		</div>

		<div class="flex min-h-0 flex-1">
			<!-- Sidebar -->
			<div class="shrink-0">
				<Sidebar
					bind:activeTab
					bind:collapsed={sidebarCollapsed}
					bind:allTabs
					bind:showSettings
					settingsInitialTab={isBillingBlocking
						? billingBlockingTab
						: isSelfHostedPlanLocked
							? licenseLockTab
							: 'account'}
					settingsDismissible={!isBillingBlocking}
					mainAppLocked={isMainAppLocked}
					showLockedMainNav={isSelfHostedPlanLocked}
					{mainAppAvailable}
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
			<main class="relative min-w-0 flex-1 overflow-auto">
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
					with everything else. `contain: paint` does the same for `position: fixed`
					descendants, such as a list's bulk-tag island.
				-->
					<!-- Main-app tabs mount only once the org is known to be unlocked; their
				     queries hit routes a licensed self-hosted org is rejected from. -->
					{#if mainAppAvailable}
						{#each allTabs as tab (tab.id)}
							{#if tab.subTabIds && tab.subTabDefs}
								<div
									class={!tab.subTabIds.includes(activeTab)
										? 'relative h-0 overflow-hidden contain-[paint]'
										: ''}
								>
									<ContentSubTabs
										tabs={tab.subTabDefs}
										bind:activeTab
										notifications={tab.subTabNotifications}
									/>
								</div>
							{:else}
								<div
									class={activeTab !== tab.id ? 'relative h-0 overflow-hidden contain-[paint]' : ''}
								>
									<tab.component isActive={activeTab === tab.id} />
								</div>
							{/if}
						{/each}
					{:else if isSelfHostedPlanLocked}
						<CloudPlanRequired page={TAB_LABELS[activeTab] ?? TAB_LABELS.topology} />
					{/if}
				</div>
			</main>
		</div>
	</div>

	<TrialExpiryModal
		blocked={isBillingBlocking || showBillingModal || $modalState.name !== null}
		bind:open={trialModalOpen}
	/>

	<!-- Searches routes a locked org is rejected from, so only with the main app. -->
	{#if mainAppAvailable}
		<GlobalSearch />
	{/if}

	<!-- Billing modal rendered last so it stacks on top of other modals -->
	<!-- Opened over a form by an upgrade badge, it takes no registry name, so the form stays mounted. -->
	<BillingPlanModal
		isOpen={showBillingModal || billingPlanOverModal}
		name={billingPlanOverModal ? undefined : 'billing-plan'}
		dismissible={!needsPlanSelection || planUnchosen}
		onClose={(selectedPlan) => {
			const overModal = billingPlanOverModal;
			if (overModal) billingOverModal.set(null);
			planJustActivated = true;
			// Key the licensed check off the plan the user just picked: `organization`
			// still holds the previous plan at this point.
			const licensed =
				selectedPlan != null && billingPlans.getMetadata(selectedPlan.type).license_plan != null;
			if (selectedPlan != null && !licensed) cloudPlanJustPicked = true;
			// Over a form, the registry still names the form, which stays open.
			if (!overModal) closeModal();
			if (licensed) {
				// Locks the app and opens Settings on the License tab now, rather than
				// when the webhook lands the plan on the org, whether the org was on a
				// cloud plan or another licensed one.
				licensedPlanJustPicked = true;
				reopenSettingsAfterBilling.set(false);
				licenseSettingsOpenedFor = selectedPlan.type;
				if (organization) markLicenseSettingsOpened(organization.id, selectedPlan.type);
				openModal('settings', { tab: licenseLockTab });
			} else if ($reopenSettingsAfterBilling) {
				reopenSettingsAfterBilling.set(false);
				openModal('settings', { tab: 'billing' });
			}
		}}
	/>

	<ReleaseNotesModal
		isOpen={showReleaseNotes}
		notes={unseenReleaseNotesQuery.data}
		onClose={() => {
			markReleaseNotesSeenMutation.mutate();
			closeModal();
		}}
	/>
{:else}
	<!-- Data still loading -->
	<Loading />
{/if}

<!-- Outside the chain above: the verification step and the terms screen
     replace the app, and their Resend, email change and accept still toast. -->
<Toast />
