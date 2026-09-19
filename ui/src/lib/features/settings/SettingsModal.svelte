<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { User, Building2, CreditCard, KeyRound, Mail, Settings, Monitor } from 'lucide-svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { hasLicensedPlan } from '$lib/features/organizations/types';
	import { isLicenseSigningAvailable, useConfigQuery } from '$lib/shared/stores/config-query';
	import { isMissingPaymentMethod } from '$lib/shared/utils/trial';
	import { modalState } from '$lib/shared/stores/modal-registry';
	import type { ModalTab } from '$lib/shared/components/layout/GenericModal.svelte';
	import AccountTab from './AccountTab.svelte';
	import OrganizationTab from './OrganizationTab.svelte';
	import BillingTab from './BillingTab.svelte';
	import LicenseTab from './LicenseTab.svelte';
	import EmailTab from './EmailTab.svelte';
	import SystemTab from './SystemTab.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import AppBanners from '$lib/shared/components/feedback/AppBanners.svelte';
	import {
		common_account,
		common_billing,
		common_email,
		common_license,
		common_organization,
		common_settings,
		common_system,
		settings_billing_ownerMustResolve
	} from '$lib/paraglide/messages';

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
		 * hasn't caught up. The License tab is where that user is being sent, so it
		 * exists from the moment this modal opens rather than one webhook later. */
		licensedPlanPending?: boolean;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();

	let currentUser = $derived(currentUserQuery.data);
	let org = $derived(organizationQuery.data);

	const configQuery = useConfigQuery();
	let isOwner = $derived(currentUser?.permissions === 'Owner');
	let isBillingEnabled = $derived(configQuery.data?.billing_enabled ?? false);
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
	let billingNeedsAttention = $derived(
		isBillingEnabled &&
			(!org?.plan ||
				org?.plan_status === 'past_due' ||
				org?.plan_status === 'paused' ||
				org?.plan_status === 'cancelled' ||
				isMissingPaymentMethod(org, isBillingEnabled))
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
		{ id: 'system', label: common_system(), icon: Monitor }
	]);

	// Filter tabs based on permissions
	let visibleTabs = $derived(
		baseTabs.filter((tab) => {
			if (tab.id === 'organization') return isOwner;
			if (tab.id === 'billing') return isOwner && isBillingEnabled;
			if (tab.id === 'license')
				return (
					isOwner &&
					isLicenseSigningOn &&
					(licensedPlanPending || (org != null && hasLicensedPlan(org)))
				);
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

<GenericModal
	{isOpen}
	title={common_settings()}
	{name}
	size="full"
	onClose={handleClose}
	onOpen={handleOpen}
	preventCloseOnClickOutside={!dismissible}
	showCloseButton={dismissible}
	tabs={visibleTabs}
	{activeTab}
	onTabChange={handleTabChange}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Settings} color="Blue" />
	{/snippet}

	{#snippet banners()}
		<!-- While this modal gates the app, the page behind it is covered and this
		     has no close button, so the app's banners have nowhere else to go. Same
		     stack, same conditions, rendered in the frame above the title. Without
		     it an unverified user cannot reach the resend link and is stuck. -->
		{#if !dismissible}
			<AppBanners />
		{/if}
	{/snippet}

	<!-- flex-1 min-h-0, not a fixed height: the panel is a flex column, and a
	     hardcoded height would not absorb the banners above the title. -->
	<div class="flex min-h-0 flex-1 flex-col">
		{#if !dismissible && !isOwner}
			<!-- Billing-blocked org (past_due / paused / self-hosted plan): only an
			     owner can see the Billing tab, so tell everyone else why they're held here. -->
			<div class="shrink-0 px-6 pt-6">
				<InlineWarning title="" body={settings_billing_ownerMustResolve()} />
			</div>
		{/if}
		{#if activeTab === 'account'}
			<AccountTab bind:subView={accountSubView} onClose={handleClose} {dismissible} />
		{:else if activeTab === 'email'}
			<EmailTab />
		{:else if activeTab === 'organization'}
			<OrganizationTab bind:subView={orgSubView} onClose={handleClose} {dismissible} />
		{:else if activeTab === 'billing'}
			<BillingTab {isOpen} onClose={handleClose} {dismissible} />
		{:else if activeTab === 'license'}
			<LicenseTab onClose={handleClose} {dismissible} />
		{:else if activeTab === 'system'}
			<SystemTab />
		{/if}
	</div>
</GenericModal>
