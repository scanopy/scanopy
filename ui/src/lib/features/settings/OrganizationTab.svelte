<script lang="ts">
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import { useCan } from '#lib/features/auth/access.ts';
	import { pushError, pushSuccess } from '#lib/shared/stores/feedback.ts';
	import InfoCard from '#lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '#lib/shared/components/data/InfoRow.svelte';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import { licenseProblemMessage } from './license-problem';
	import {
		useOrganizationQuery,
		useUpdateOrganizationMutation,
		useResetOrganizationDataMutation,
		usePopulateDemoDataMutation,
		useDeleteOrganizationMutation,
		fetchDemoPopulateStatus
	} from '#lib/features/organizations/queries.ts';
	import { queryClient } from '#lib/api/query-client.ts';
	import ConfirmationDialog from '#lib/shared/components/feedback/ConfirmationDialog.svelte';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { hasLicensedPlan } from '#lib/features/organizations/types.ts';
	import { resolve } from '$app/paths';
	import { formatLongDate, formatTimestamp } from '#lib/shared/utils/formatting.ts';
	import { createForm } from '@tanstack/svelte-form';
	import { required, max } from '#lib/shared/components/forms/validators.ts';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import { useModalActions } from '#lib/shared/components/layout/modalActionsContext.svelte.ts';
	import {
		common_back,
		common_created,
		common_edit,
		common_id,
		common_loading,
		common_entityName,
		common_name,
		common_never,
		common_offline,
		common_online,
		common_organization,
		common_plan,
		common_populate,
		common_populating,
		common_reset,
		common_saveChanges,
		common_saving,
		common_delete,
		common_tryAgainLater,
		settings_org_delete,
		settings_org_deleteConfirm,
		settings_org_deleteHelp,
		settings_org_deleteSuccess,
		settings_org_deleteTypeName,
		settings_org_info,
		settings_billing_license_installLinkText,
		settings_billing_license_lastCheckIn,
		settings_billing_license_validThrough,
		settings_org_communityLicenseHint,
		settings_org_licenseKeyType,
		settings_org_licenseStatus,
		settings_org_namePlaceholder,
		settings_org_populateConfirm,
		settings_org_populateDemo,
		settings_org_populateDemoHelp,
		settings_org_populateFailed,
		settings_org_populateSuccess,
		settings_org_resetConfirm,
		settings_org_resetData,
		settings_org_resetDataHelp,
		settings_org_resetSuccess,
		settings_org_unableToLoad,
		settings_org_updateName,
		settings_org_updated
	} from '#lib/paraglide/messages.js';

	let {
		subView = $bindable<'main' | 'edit'>('main')
	}: {
		subView?: 'main' | 'edit';
	} = $props();

	// TanStack Query for current user

	// TanStack Query for organization
	const organizationQuery = useOrganizationQuery();
	const updateOrganizationMutation = useUpdateOrganizationMutation();
	const resetOrganizationDataMutation = useResetOrganizationDataMutation();
	const populateDemoDataMutation = usePopulateDemoDataMutation();
	const deleteOrganizationMutation = useDeleteOrganizationMutation();

	let saving = $derived(updateOrganizationMutation.isPending);
	let resetting = $derived(resetOrganizationDataMutation.isPending);
	// Populate runs in the background: the POST resolves at the 202, so the
	// button must stay busy across the status poll, tracked separately.
	let demoPolling = $state(false);
	let populating = $derived(populateDemoDataMutation.isPending || demoPolling);
	let deleting = $derived(deleteOrganizationMutation.isPending);
	let showDeleteConfirm = $state(false);

	const configQuery = useConfigQuery();
	const can = useCan();

	let org = $derived(organizationQuery.data);
	let isOwner = $derived(can('Organization', 'Write'));
	let isDemoOrg = $derived(billingPlans.getMetadata(org?.plan?.type ?? null).is_demo === true);
	// Reset and populate-demo hit main-app routes, which stay locked on a licensed
	// plan.
	let mainAppRoutesLocked = $derived(org != null && hasLicensedPlan(org));

	// TanStack Form
	const form = createForm(() => ({
		defaultValues: {
			name: org?.name ?? ''
		},
		onSubmit: async ({ value }) => {
			await handleSave(value.name);
		}
	}));

	// Reset form when switching to edit view
	export function resetForm() {
		if (org) {
			form.reset();
			form.setFieldValue('name', org.name);
		}
	}

	async function handleSave(name: string) {
		if (!org) return;

		try {
			await updateOrganizationMutation.mutateAsync({ id: org.id, name });
			pushSuccess(settings_org_updated());
			subView = 'main';
		} catch {
			// The API client reports the failure.
		}
	}

	function handleBack() {
		subView = 'main';
		if (org) {
			form.setFieldValue('name', org.name);
		}
	}

	async function handleReset() {
		if (!org) return;

		if (!confirm(settings_org_resetConfirm())) {
			return;
		}

		try {
			await resetOrganizationDataMutation.mutateAsync(org.id);
			pushSuccess(settings_org_resetSuccess());
		} catch {
			// The API client reports the failure.
		}
	}

	async function handleDelete() {
		if (!org) return;

		try {
			await deleteOrganizationMutation.mutateAsync(org.id);
			pushSuccess(settings_org_deleteSuccess());
			// A full load, as logout does: `queryClient.clear()` orphans the queries of
			// components that stay mounted across a client-side navigation (AppShell's
			// banners and payment dialog), which would keep showing the deleted org.
			window.location.assign(resolve('onboarding'));
		} catch {
			// API client middleware auto-toasts the translated error message
			// (e.g. "Cancel your subscription before deleting your organization").
			showDeleteConfirm = false;
		}
	}

	async function handlePopulateDemo() {
		if (!org) return;

		if (!confirm(settings_org_populateConfirm())) {
			return;
		}

		const orgId = org.id;
		demoPolling = true;
		try {
			// 202 kicks off the background task; poll its status until terminal.
			await populateDemoDataMutation.mutateAsync(orgId);

			// Poll every 1.5s, capped at ~5 minutes so a stuck task can't spin
			// the button forever.
			const maxAttempts = 200;
			let terminal: 'complete' | 'failed' | null = null;
			for (let attempt = 0; attempt < maxAttempts; attempt++) {
				await new Promise((resolve) => setTimeout(resolve, 1500));
				const status = await fetchDemoPopulateStatus(orgId);
				if (status.state !== 'running') {
					terminal = status.state;
					break;
				}
			}

			if (terminal === 'complete') {
				await queryClient.invalidateQueries();
				pushSuccess(settings_org_populateSuccess());
			} else {
				pushError(settings_org_populateFailed());
			}
		} catch {
			// The API client reports the failure. The `else` above stays: a poll that
			// times out without a terminal state throws nothing, so nothing else
			// would report it.
		} finally {
			demoPolling = false;
		}
	}

	// Only the edit view has a footer; the main view closes from the header.
	let actions = $derived<ModalAction[]>(
		subView === 'edit'
			? [
					{ role: 'secondary', label: common_back(), onClick: handleBack },
					{
						role: 'primary',
						label: saving ? common_saving() : common_saveChanges(),
						onClick: () => form.handleSubmit(),
						disabled: saving
					}
				]
			: []
	);
	useModalActions(() => actions);
</script>

<div class="flex min-h-0 flex-1 flex-col">
	{#if org}
		{#if subView === 'main'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<!-- Organization Info -->
					<InfoCard title={settings_org_info()}>
						<InfoRow label={common_name()}>{org.name}</InfoRow>
						<InfoRow label={common_created()}>
							{formatTimestamp(org.created_at)}
						</InfoRow>
						<InfoRow label={common_id()} mono={true}>{org.id}</InfoRow>
						{#if org.plan}
							<InfoRow label={common_plan()}>{billingPlans.getName(org.plan.type)}</InfoRow>
						{/if}
						{#if configQuery.data?.license_valid_through}
							<InfoRow label={settings_billing_license_validThrough()}>
								{formatLongDate(configQuery.data.license_valid_through)}
							</InfoRow>
						{/if}
						{#if configQuery.data?.license_key_type}
							<InfoRow label={settings_org_licenseKeyType()}>
								{configQuery.data.license_key_type === 'Online'
									? common_online()
									: common_offline()}
							</InfoRow>
						{/if}
						{#if configQuery.data?.license_problem}
							<InfoRow label={settings_org_licenseStatus()}>
								{licenseProblemMessage(configQuery.data.license_problem)}
							</InfoRow>
						{/if}
						{#if configQuery.data?.deployment_type === 'community'}
							<DocsHint
								class="mt-2"
								text={settings_org_communityLicenseHint()}
								href="https://scanopy.net/docs/self-hosted-server/installation/?edition=commercial"
								linkText={settings_billing_license_installLinkText()}
							/>
						{/if}
						{#if configQuery.data?.license_key_type === 'Online'}
							<InfoRow label={settings_billing_license_lastCheckIn()}>
								{configQuery.data.license_entitlement_at
									? formatTimestamp(configQuery.data.license_entitlement_at)
									: common_never()}
							</InfoRow>
						{/if}
					</InfoCard>

					<!-- Actions -->
					<InfoCard>
						<div class="flex items-center justify-between">
							<div>
								<p class="text-sm font-medium text-primary">
									{common_entityName({ entity: common_organization() })}
								</p>
								<p class="text-xs text-secondary">{settings_org_updateName()}</p>
							</div>
							<button
								onclick={() => {
									subView = 'edit';
									form.setFieldValue('name', org.name);
								}}
								class="btn-primary"
							>
								{common_edit()}
							</button>
						</div>
					</InfoCard>

					{#if isOwner}
						{#if !mainAppRoutesLocked}
							{#if isDemoOrg}
								<!-- Populate Demo Data (only for Demo orgs) -->
								<InfoCard>
									<div class="flex items-center justify-between">
										<div>
											<p class="text-sm font-medium text-primary">{settings_org_populateDemo()}</p>
											<p class="text-xs text-secondary">
												{settings_org_populateDemoHelp()}
											</p>
										</div>
										<button onclick={handlePopulateDemo} disabled={populating} class="btn-primary">
											{populating ? common_populating() : common_populate()}
										</button>
									</div>
								</InfoCard>
							{/if}

							<!-- Reset Organization Data (available to all org owners) -->
							<InfoCard>
								<div class="flex items-center justify-between">
									<div>
										<p class="text-sm font-medium text-primary">{settings_org_resetData()}</p>
										<p class="text-xs text-secondary">
											{settings_org_resetDataHelp()}
										</p>
									</div>
									<button onclick={handleReset} disabled={resetting} class="btn-danger">
										{resetting ? common_loading() : common_reset()}
									</button>
								</div>
							</InfoCard>
						{/if}

						{#if !isDemoOrg}
							<!-- Delete Organization (not offered on Demo orgs) -->
							<InfoCard>
								<div class="flex items-center justify-between">
									<div>
										<p class="text-sm font-medium text-primary">{settings_org_delete()}</p>
										<p class="text-xs text-secondary">
											{settings_org_deleteHelp()}
										</p>
									</div>
									<button
										onclick={() => (showDeleteConfirm = true)}
										disabled={deleting}
										class="btn-danger"
									>
										{deleting ? common_loading() : common_delete()}
									</button>
								</div>
							</InfoCard>
						{/if}
					{/if}
				</div>
			</div>
		{:else if subView === 'edit'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<p class="text-sm text-secondary">{settings_org_updateName()}</p>
					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<TextInput
								label={common_entityName({ entity: common_organization() })}
								id="name"
								placeholder={settings_org_namePlaceholder()}
								required={true}
								{field}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>
		{/if}
	{:else}
		<div class="flex-1 overflow-auto p-6">
			<div class="py-8 text-center text-secondary">
				<p>{settings_org_unableToLoad()}</p>
				<p class="mt-2 text-sm text-tertiary">{common_tryAgainLater()}</p>
			</div>
		</div>
	{/if}
</div>

{#if org}
	<ConfirmationDialog
		isOpen={showDeleteConfirm}
		title={settings_org_delete()}
		message={settings_org_deleteConfirm()}
		confirmLabel={settings_org_delete()}
		cancelLabel={common_back()}
		variant="danger"
		confirmText={org.name}
		confirmPlaceholder={settings_org_deleteTypeName()}
		onConfirm={handleDelete}
		onCancel={() => (showDeleteConfirm = false)}
		onClose={() => (showDeleteConfirm = false)}
	/>
{/if}
