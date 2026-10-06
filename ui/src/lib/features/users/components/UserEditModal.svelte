<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import PermissionSelect from '$lib/shared/components/api-keys/PermissionSelect.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { SiteDisplay } from '$lib/shared/components/forms/selection/display/SiteDisplay.svelte';
	import { entities, permissions, metadata } from '$lib/shared/stores/metadata';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useUpdateUserAsAdminMutation } from '$lib/features/users/queries';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import type { User, UserOrgPermissions } from '../types';
	import type { Site } from '$lib/features/sites/types';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import {
		common_access,
		common_account,
		common_authentication,
		common_cancel,
		common_email,
		common_emailAndPassword,
		common_sites,
		common_saveChanges,
		common_saving,
		users_editUser,
		users_editUserTitle,
		users_hasAllSites,
		users_siteAccessHelp,
		users_permissionsLevel,
		users_permissionsLevelHelp,
		users_updateSuccess
	} from '$lib/paraglide/messages';

	let {
		isOpen = $bindable(false),
		user,
		onClose,
		name = undefined
	}: {
		isOpen: boolean;
		user: User | null;
		onClose: () => void;
		name?: string;
	} = $props();

	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);

	// TanStack Query mutation for updating user
	const updateUserMutation = useUpdateUserAsAdminMutation();

	// Force Svelte to track reactivity
	$effect(() => {
		void $metadata;
	});

	let loading = $derived(updateUserMutation.isPending);

	// Permission levels that don't need site assignment
	const sitesNotNeeded: string[] = permissions
		.getItems()
		.filter((p) => p.metadata.manage_org_entities)
		.map((p) => p.id);

	// Selected sites state
	let selectedSites: Site[] = $state([]);

	// Available sites for selection
	let siteOptions = $derived(sitesData.filter((n) => !selectedSites.some((sn) => sn.id === n.id)));

	function getDefaultValues() {
		return {
			permissions: user?.permissions || ('Viewer' as UserOrgPermissions)
		};
	}

	// Create form
	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			if (!user) return;

			try {
				const updatedUser: User = {
					...user,
					permissions: value.permissions as UserOrgPermissions,
					site_ids: sitesNotNeeded.includes(value.permissions as UserOrgPermissions)
						? []
						: selectedSites.map((n) => n.id)
				};

				await updateUserMutation.mutateAsync(updatedUser);
				pushSuccess(users_updateSuccess({ email: user.email }));
				onClose();
			} catch {
				// The API client reports the failure.
			}
		}
	}));

	// Mirrored from the form store: form.state.values is not tracked by $derived.
	let permissionsValue = $state<string>(getDefaultValues().permissions);
	$effect(() => {
		return form.store.subscribe(() => {
			permissionsValue = form.state.values.permissions;
		});
	});

	// Reset form when modal opens
	function handleOpen() {
		form.reset(getDefaultValues());
		if (user) {
			selectedSites = user.site_ids
				.map((id) => sitesData.find((n) => n.id === id))
				.filter((n): n is Site => n !== undefined);
		} else {
			selectedSites = [];
		}
	}

	function handleAddSite(id: string) {
		const site = sitesData.find((n) => n.id === id);
		if (site) {
			selectedSites = [...selectedSites, site];
		}
	}

	function handleRemoveSite(index: number) {
		selectedSites = selectedSites.filter((_, i) => i !== index);
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleClose() {
		if (!loading) {
			onClose();
		}
	}

	let title = $derived(user ? users_editUser({ email: user.email }) : users_editUserTitle());
</script>

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={user?.id}
	{form}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon
			Icon={entities.getIconComponent('User')}
			color={entities.getColorHelper('User').color}
		/>
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			{#if user}
				<div class="space-y-6">
					<InfoCard title={common_account()}>
						<div class="space-y-2">
							<div class="flex items-center justify-between">
								<span class="text-secondary text-sm">{common_email()}</span>
								<span class="text-primary text-sm font-medium">{user.email}</span>
							</div>
							<div class="flex items-center justify-between">
								<span class="text-secondary text-sm">{common_authentication()}</span>
								<span class="text-primary text-sm"
									>{user.oidc_provider || common_emailAndPassword()}</span
								>
							</div>
						</div>
					</InfoCard>

					<InfoCard title={common_access()}>
						<form.Field
							name="permissions"
							validators={{
								onChange: ({ value }) => required(value)
							}}
						>
							{#snippet children(field)}
								<PermissionSelect
									{field}
									label={users_permissionsLevel()}
									context="user"
									helpText={users_permissionsLevelHelp()}
								/>
							{/snippet}
						</form.Field>

						<!-- Site Assignment (only for Member/Viewer) -->
						{#if !sitesNotNeeded.includes(permissionsValue as UserOrgPermissions)}
							<ListManager
								label={common_sites()}
								helpText={users_siteAccessHelp()}
								required={true}
								allowReorder={false}
								allowAddFromOptions={true}
								allowCreateNew={false}
								allowItemEdit={() => false}
								disableCreateNewButton={false}
								onAdd={handleAddSite}
								onRemove={handleRemoveSite}
								options={siteOptions}
								optionDisplayComponent={SiteDisplay}
								items={selectedSites}
								itemDisplayComponent={SiteDisplay}
							/>
						{:else}
							<InlineInfo body={users_hasAllSites({ permissions: permissionsValue })} />
						{/if}
					</InfoCard>
				</div>
			{/if}
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" disabled={loading} onclick={handleClose} class="btn-secondary">
					{common_cancel()}
				</button>
				<button type="submit" disabled={loading} class="btn-primary">
					{loading ? common_saving() : common_saveChanges()}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
