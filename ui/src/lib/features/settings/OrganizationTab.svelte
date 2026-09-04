<script lang="ts">
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		useOrganizationQuery,
		useUpdateOrganizationMutation,
		useResetOrganizationDataMutation,
		usePopulateDemoDataMutation,
		useDeleteOrganizationMutation,
		fetchDemoPopulateStatus
	} from '$lib/features/organizations/queries';
	import { queryClient } from '$lib/api/query-client';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { createForm } from '@tanstack/svelte-form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_back,
		common_close,
		common_created,
		common_edit,
		common_id,
		common_loading,
		common_entityName,
		common_name,
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
		settings_org_namePlaceholder,
		settings_org_populateConfirm,
		settings_org_populateDemo,
		settings_org_populateDemoHelp,
		settings_org_populateFailed,
		settings_org_populateSuccess,
		settings_org_resetConfirm,
		settings_org_resetData,
		settings_org_resetDataHelp,
		settings_org_resetFailed,
		settings_org_resetSuccess,
		settings_org_unableToLoad,
		settings_org_updateFailed,
		settings_org_updateName,
		settings_org_updated
	} from '$lib/paraglide/messages';

	let {
		subView = $bindable<'main' | 'edit'>('main'),
		onClose,
		dismissible = true
	}: {
		subView?: 'main' | 'edit';
		onClose: () => void;
		/** When false (hard-gated modal), the main-view Close button is hidden so
		 * the gate can't be escaped from this tab. The edit sub-view keeps its
		 * Back/Save footer. */
		dismissible?: boolean;
	} = $props();

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

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

	let org = $derived(organizationQuery.data);
	let isOwner = $derived(currentUser?.permissions === 'Owner');
	let isDemoOrg = $derived(billingPlans.getMetadata(org?.plan?.type ?? null).is_demo === true);

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
			pushError(settings_org_updateFailed());
		}
	}

	function handleCancel() {
		if (subView === 'edit') {
			subView = 'main';
			if (org) {
				form.setFieldValue('name', org.name);
			}
		} else {
			onClose();
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
			pushError(settings_org_resetFailed());
		}
	}

	async function handleDelete() {
		if (!org) return;

		try {
			await deleteOrganizationMutation.mutateAsync(org.id);
			pushSuccess(settings_org_deleteSuccess());
			await goto(resolve('/onboarding'));
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
			pushError(settings_org_populateFailed());
		} finally {
			demoPolling = false;
		}
	}

	let showSave = $derived(subView === 'edit');
	let cancelLabel = $derived(subView === 'main' ? common_close() : common_back());
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
							<InfoRow label={common_plan()}>{org.plan.type}</InfoRow>
						{/if}
					</InfoCard>

					<!-- Actions -->
					<InfoCard>
						<div class="flex items-center justify-between">
							<div>
								<p class="text-primary text-sm font-medium">
									{common_entityName({ entity: common_organization() })}
								</p>
								<p class="text-secondary text-xs">{settings_org_updateName()}</p>
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
						<!-- Reset Organization Data (available to all org owners) -->
						<InfoCard>
							<div class="flex items-center justify-between">
								<div>
									<p class="text-primary text-sm font-medium">{settings_org_resetData()}</p>
									<p class="text-secondary text-xs">
										{settings_org_resetDataHelp()}
									</p>
								</div>
								<button onclick={handleReset} disabled={resetting} class="btn-danger">
									{resetting ? common_loading() : common_reset()}
								</button>
							</div>
						</InfoCard>

						{#if isDemoOrg}
							<!-- Populate Demo Data (only for Demo orgs) -->
							<InfoCard>
								<div class="flex items-center justify-between">
									<div>
										<p class="text-primary text-sm font-medium">{settings_org_populateDemo()}</p>
										<p class="text-secondary text-xs">
											{settings_org_populateDemoHelp()}
										</p>
									</div>
									<button onclick={handlePopulateDemo} disabled={populating} class="btn-primary">
										{populating ? common_populating() : common_populate()}
									</button>
								</div>
							</InfoCard>
						{/if}

						<!-- Delete Organization -->
						<InfoCard>
							<div class="flex items-center justify-between">
								<div>
									<p class="text-primary text-sm font-medium">{settings_org_delete()}</p>
									<p class="text-secondary text-xs">
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
				</div>
			</div>
		{:else if subView === 'edit'}
			<div class="flex-1 overflow-auto p-6">
				<div class="space-y-6">
					<p class="text-secondary text-sm">{settings_org_updateName()}</p>
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
			<div class="text-secondary py-8 text-center">
				<p>{settings_org_unableToLoad()}</p>
				<p class="text-tertiary mt-2 text-sm">{common_tryAgainLater()}</p>
			</div>
		</div>
	{/if}

	<!-- Footer — hidden on the main view when the modal is hard-gated
	     (dismissible=false), since the only button there is Close, which would
	     let the user escape the gate. The edit sub-view keeps its Back/Save. -->
	{#if dismissible || subView !== 'main'}
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={handleCancel} class="btn-secondary">
					{cancelLabel}
				</button>
				{#if showSave}
					<button
						type="button"
						onclick={() => form.handleSubmit()}
						disabled={saving}
						class="btn-primary"
					>
						{saving ? common_saving() : common_saveChanges()}
					</button>
				{/if}
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
