<script lang="ts">
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import {
		User,
		Building2,
		CreditCard,
		KeyRound,
		Mail,
		Settings,
		Monitor,
		LifeBuoy
	} from '@lucide/svelte';
	import { useCan } from '#lib/features/auth/access.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import {
		hasLicensedPlan,
		isPlanLapsed,
		isPlanUnchosen,
		isSelfHostedPlanLocked
	} from '#lib/features/organizations/types.ts';
	import { useHasPendingQuote } from '#lib/features/billing/queries.ts';
	import {
		isCloud,
		isLicenseSigningAvailable,
		useConfigQuery
	} from '#lib/shared/stores/config-query.ts';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { licenseTabVisible } from './license-tab';
	import { isMissingPaymentMethod } from '#lib/shared/utils/trial.ts';
	import { modalState } from '#lib/shared/stores/modal-registry.ts';
	import type { ModalTab } from '#lib/shared/components/layout/GenericModal.svelte';
	import AccountTab from './AccountTab.svelte';
	import OrganizationTab from './OrganizationTab.svelte';
	import BillingTab from './BillingTab.svelte';
	import LicenseTab from './LicenseTab.svelte';
	import EmailTab from './EmailTab.svelte';
	import SystemTab from './SystemTab.svelte';
	import SupportTab from './SupportTab.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import AppBanners from '#lib/shared/components/feedback/AppBanners.svelte';
	import {
		common_account,
		common_billing,
		common_email,
		common_license,
		common_organization,
		common_settings,
		common_support,
		common_system,
		settings_billing_ownerHoldsLicense,
		settings_billing_ownerMustResolve
	} from '#lib/paraglide/messages.js';

	let {
		isOpen = false,
		onClose,
		initialTab = 'account',
		dismissible = true,
		name = undefined,
		licensedPlanPending = false
	}: {
		isOpen: boolean;
		onClose: () => void;
		initialTab?: string;
		dismissible?: boolean;
		name?: string;
		/** A licensed self-hosted plan was just picked and the organization query
		 * hasn't caught up. The License tab shows the key rather than the cloud-plan
		 * preview from the moment this modal opens rather than one webhook later. */
		licensedPlanPending?: boolean;
	} = $props();

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();

	let org = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	const can = useCan();
	let canManageBilling = $derived(can('Billing', 'Write'));
	let isCloudDeployment = $derived(configQuery.data ? isCloud(configQuery.data) : false);
	let isDemoOrg = $derived(billingPlans.getMetadata(org?.plan?.type ?? null).is_demo === true);
	// No signing key means the mint endpoints fail, so the tab would only produce
	// an error toast.
	let isLicenseSigningOn = $derived(
		configQuery.data != null && isLicenseSigningAvailable(configQuery.data)
	);
	// Payment-prompt cases use the shared isMissingPaymentMethod predicate so
	// this tab dot stays in sync with the sidebar billing dot (and the banner /
	// BillingTab card). The other clauses are broader billing-attention states
	// (no plan yet, paused, cancelled) that are specific to this tab.
	// Every clause reads org rows that only Stripe webhooks write, so they go
	// stale on a deployment with billing switched off. The tab itself is hidden
	// there, and the dot follows it.
	//
	// An org with a quote out is already arranging payment, so the missing card
	// lights nothing, as the banners stand down for it too.
	const hasPendingQuote = useHasPendingQuote();
	let billingNeedsAttention = $derived(
		isCloudDeployment &&
			(isPlanUnchosen(org, isCloudDeployment) ||
				org?.plan_status === 'past_due' ||
				org?.plan_status === 'paused' ||
				org?.plan_status === 'cancelled' ||
				(isMissingPaymentMethod(org, isCloudDeployment) && !hasPendingQuote.current))
	);

	// An org on a licensed plan reaches only Settings, so the app's banners show
	// here rather than over the pages behind it.
	let licenseLocked = $derived(
		(isCloudDeployment && licensedPlanPending) || isSelfHostedPlanLocked(org, isCloudDeployment)
	);

	// Tab and sub-view state
	let activeTab = $state('account');
	let accountSubView = $state<'main' | 'credentials' | 'cookies'>('main');
	let orgSubView = $state<'main' | 'edit'>('main');

	// Define base tabs
	let baseTabs = $derived<ModalTab[]>([
		{ id: 'account', label: common_account(), icon: User },
		{ id: 'email', label: common_email(), icon: Mail },
		{ id: 'organization', label: common_organization(), icon: Building2 },
		{
			id: 'billing',
			label: common_billing(),
			icon: CreditCard,
			notification: billingNeedsAttention
		},
		{ id: 'license', label: common_license(), icon: KeyRound },
		{ id: 'system', label: common_system(), icon: Monitor },
		{ id: 'support', label: common_support(), icon: LifeBuoy }
	]);

	// Filter tabs based on permissions
	let visibleTabs = $derived(
		baseTabs.filter((tab) => {
			if (tab.id === 'organization') return can('Organization', 'Write');
			if (tab.id === 'billing') return can('Billing', 'Read') && isCloudDeployment;
			if (tab.id === 'license')
				return licenseTabVisible({
					isOwner: can('License', 'Read'),
					isDemoOrg,
					isCloudDeployment,
					signingAvailable: isLicenseSigningOn
				});
			return true;
		})
	);

	// A caller can target a tab that isn't visible yet — the lock effect asks for
	// License while the organization query still holds the previous plan, and
	// GenericModal falls back to the first tab when the requested one is missing.
	// Apply the requested tab once it exists.
	$effect(() => {
		const requested = $modalState.name === 'settings' ? $modalState.tab : null;
		if (
			isOpen &&
			requested &&
			requested !== activeTab &&
			visibleTabs.some((tab) => tab.id === requested)
		) {
			activeTab = requested;
		}
	});

	// Reset sub-views when modal opens or tab changes
	function handleOpen() {
		activeTab = initialTab;
		accountSubView = 'main';
		orgSubView = 'main';
	}

	function handleTabChange(tabId: string) {
		activeTab = tabId;
		// Reset sub-views when switching tabs
		accountSubView = 'main';
		orgSubView = 'main';
	}

	function handleClose() {
		if (!dismissible) return;
		// Reset sub-views on close
		accountSubView = 'main';
		orgSubView = 'main';
		onClose();
	}
</script>

<!-- Declared out here, not as a child of the modal, so it can be passed
     conditionally. While this modal gates the app, the page behind it is covered
     and this has no close button, so the app's banners have nowhere else to go.
     The title row stands down with them, so the banners take its place. A
     license-locked org keeps its title row and close button under them. -->
{#snippet bannersSnippet()}
	<AppBanners />
{/snippet}

<GenericModal
	{isOpen}
	title={common_settings()}
	{name}
	size="full"
	onClose={handleClose}
	onOpen={handleOpen}
	preventCloseOnClickOutside={!dismissible}
	showCloseButton={dismissible}
	showTitleRow={dismissible}
	banners={dismissible && !licenseLocked ? undefined : bannersSnippet}
	tabs={visibleTabs}
	{activeTab}
	onTabChange={handleTabChange}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Settings} color="Blue" />
	{/snippet}

	<!-- flex-1 min-h-0, not a fixed height: the panel is a flex column, and a
	     hardcoded height would not absorb the banners above the title. -->
	<div class="flex min-h-0 flex-1 flex-col">
		{#if (!dismissible || licenseLocked) && !canManageBilling && !(org != null && isPlanLapsed(org))}
			<!-- Billing-blocked org (past_due / paused), or one on a self-hosted plan: only
			     an owner can see the Billing and License tabs, so tell everyone else why.
			     A lapsed org's banner already says so and asks for an owner. -->
			<div class="shrink-0 px-6 pt-6">
				<InlineWarning
					title=""
					body={org != null && hasLicensedPlan(org)
						? settings_billing_ownerHoldsLicense()
						: settings_billing_ownerMustResolve()}
				/>
			</div>
		{/if}
		{#if activeTab === 'account'}
			<AccountTab bind:subView={accountSubView} onClose={handleClose} />
		{:else if activeTab === 'email'}
			<EmailTab />
		{:else if activeTab === 'organization'}
			<OrganizationTab bind:subView={orgSubView} />
		{:else if activeTab === 'billing'}
			<BillingTab {isOpen} onClose={handleClose} />
		{:else if activeTab === 'license'}
			<LicenseTab onClose={handleClose} {licensedPlanPending} />
		{:else if activeTab === 'system'}
			<SystemTab />
		{:else if activeTab === 'support'}
			<SupportTab />
		{/if}
	</div>
</GenericModal>
