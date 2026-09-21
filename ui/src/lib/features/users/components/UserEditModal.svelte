<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
	import { entities, permissions, metadata } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useNetworksQuery } from '$lib/features/networks/queries';
	import { useUpdateUserAsAdminMutation } from '$lib/features/users/queries';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import type { User, UserOrgPermissions } from '../types';
	import type { Network } from '$lib/features/networks/types';
	import {
		common_authentication,
		common_cancel,
		common_email,
		common_emailAndPassword,
		common_networks,
		common_saveChanges,
		common_saving,
		users_editUser,
		users_editUserTitle,
		users_hasAllNetworks,
		users_networkAccessHelp,
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

	// TanStack Query for current user
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const networksQuery = useNetworksQuery();
	let networksData = $derived(networksQuery.data ?? []);

	// TanStack Query mutation for updating user
	const updateUserMutation = useUpdateUserAsAdminMutation();

	// Force Svelte to track reactivity
	$effect(() => {
		void $metadata;
		void currentUser;
	});

	let loading = $derived(updateUserMutation.isPending);

	// Permission levels that don't need network assignment
	const networksNotNeeded: string[] = permissions
		.getItems()
		.filter((p) => p.metadata.manage_org_entities)
		.map((p) => p.id);

	// Selected networks state
	let selectedNetworks: Network[] = $state([]);

	// Filter permission options to only those the current user can manage
	let permissionOptions = $derived(
		permissions
			.getItems()
			.filter((p) => {
				if (!currentUser) return false;
				const canManage =
					permissions
						.getMetadata(currentUser.permissions)
						?.grantable_user_permissions?.includes(p.id) ?? false;
				return canManage;
			})
			.map((p) => ({ value: p.id, label: p.name ?? '', description: p.description ?? '' }))
	);

	// Available networks for selection
	let networkOptions = $derived(
		networksData.filter((n) => !selectedNetworks.some((sn) => sn.id === n.id))
	);

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
					network_ids: networksNotNeeded.includes(value.permissions as UserOrgPermissions)
						? []
						: selectedNetworks.map((n) => n.id)
				};

				await updateUserMutation.mutateAsync(updatedUser);
				pushSuccess(users_updateSuccess({ email: user.email }));
				onClose();
			} catch {
				// The API client reports the failure.
			}
		}
	}));

	let permissionsValue = $derived(form.state.values.permissions);

	// Reset form when modal opens
	function handleOpen() {
		form.reset(getDefaultValues());
		if (user) {
			selectedNetworks = user.network_ids
				.map((id) => networksData.find((n) => n.id === id))
				.filter((n): n is Network => n !== undefined);
		} else {
			selectedNetworks = [];
		}
	}

	function handleAddNetwork(id: string) {
		const network = networksData.find((n) => n.id === id);
		if (network) {
			selectedNetworks = [...selectedNetworks, network];
		}
	}

	function handleRemoveNetwork(index: number) {
		selectedNetworks = selectedNetworks.filter((_, i) => i !== index);
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
					<!-- User Info (read-only) -->
					<div class="card card-static">
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
					</div>

					<!-- Permissions Selection -->
					<form.Field
						name="permissions"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<SelectInput
								label={users_permissionsLevel()}
								id="permissions"
								{field}
								options={permissionOptions}
								helpText={users_permissionsLevelHelp()}
							/>
						{/snippet}
					</form.Field>

					<!-- Network Assignment (only for Member/Viewer) -->
					{#if !networksNotNeeded.includes(permissionsValue as UserOrgPermissions)}
						<ListManager
							label={common_networks()}
							helpText={users_networkAccessHelp()}
							required={true}
							allowReorder={false}
							allowAddFromOptions={true}
							allowCreateNew={false}
							allowItemEdit={() => false}
							disableCreateNewButton={false}
							onAdd={handleAddNetwork}
							onRemove={handleRemoveNetwork}
							options={networkOptions}
							optionDisplayComponent={NetworkDisplay}
							items={selectedNetworks}
							itemDisplayComponent={NetworkDisplay}
						/>
					{:else}
						<div class="card card-static">
							<p class="text-secondary text-sm">
								{users_hasAllNetworks({ permissions: permissionsValue })}
							</p>
						</div>
					{/if}
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
