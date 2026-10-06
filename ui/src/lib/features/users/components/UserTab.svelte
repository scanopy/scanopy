<script lang="ts">
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { siteItems } from '$lib/features/sites/columns';
	import type { LabelledCardFieldItem } from '$lib/shared/components/data/types';
	import { Edit, UserX, Trash2 } from 'lucide-svelte';
	import type { CardAction } from '$lib/shared/components/data/types';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '$lib/shared/components/data/types';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import {
		useInvitesQuery,
		formatInviteUrl,
		useRevokeInviteMutation
	} from '$lib/features/organizations/queries';
	import { UserPlus } from 'lucide-svelte';
	import { isUser, type User, type UserOrInvite } from '../types';
	import type { OrganizationInvite } from '$lib/features/organizations/types';
	import InviteModal from './InviteModal.svelte';
	import { metadata, permissions, entities } from '$lib/shared/stores/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import UserEditModal from './UserEditModal.svelte';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useUsersQuery, useBulkDeleteUsersMutation, useDeleteUserMutation } from '../queries';
	import type { TabProps } from '$lib/shared/types';
	import { downloadCsv } from '$lib/shared/utils/csvExport';
	import { modalState, resolveModalDeepLink } from '$lib/shared/stores/modal-registry';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import {
		common_all,
		common_sites,
		common_edit,
		common_confirmBulkDelete,
		common_delete,
		common_email,
		common_emailAndPassword,
		common_none,
		common_expires,
		common_joined,
		common_revoke,
		common_role,
		common_status,
		common_unknownEntity,
		common_updated,
		common_url,
		common_user,
		common_users,
		common_you,
		invites_confirmRevoke,
		invites_createdBy,
		invites_pendingInvite,
		users_authMethod,
		users_authMethodOidcAndPassword,
		users_confirmDeleteUser,
		users_emailVerified,
		users_inviteUser,
		users_noUsersFound,
		users_noUsersSubtitle,
		users_providerLinked,
		users_subtitle,
		users_termsAccepted,
		users_verifyEmailToInvite
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	// Query
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let seatLimit = $derived(org?.plan?.included_seats ?? null);
	let canBuyMoreSeats = $derived(
		org?.plan?.seat_cents !== undefined && org?.plan?.seat_cents !== null
	);

	const usersQuery = useUsersQuery();
	const bulkDeleteUsersMutation = useBulkDeleteUsersMutation();
	const invitesQuery = useInvitesQuery();

	// Derived data
	let usersData = $derived(usersQuery.data ?? []);
	let invitesData = $derived(invitesQuery.data ?? []);
	let isLoading = $derived(usersQuery.isPending);

	// Force Svelte to track metadata reactivity
	$effect(() => {
		void $metadata;
	});

	let showInviteModal = $state(false);
	let showEditModal = $state(false);
	let editingUser = $state<User | null>(null);

	// Deep-link: open invite modal from URL
	$effect(() => {
		if ($modalState.name === 'invite-user' && !showInviteModal) {
			showInviteModal = true;
		}
	});

	// Deep-link: open user editor from URL (handles both fresh open and entity switch)
	$effect(() => {
		const result = resolveModalDeepLink(
			$modalState,
			'user-editor',
			usersData,
			showEditModal,
			editingUser?.id
		);
		if (result !== undefined) {
			editingUser = result;
			showEditModal = true;
		}
	});

	// Combine users and invites into single array
	let combinedItems = $derived([
		...usersData.map((user) => ({ type: 'user' as const, data: user, id: user.id })),
		...invitesData.map((invite) => ({ type: 'invite' as const, data: invite, id: invite.id }))
	] as UserOrInvite[]);

	let userCount = $derived(combinedItems.filter(isUser).length);
	let isAtSeatLimit = $derived(seatLimit !== null && userCount >= seatLimit && !canBuyMoreSeats);

	async function handleCreateInvite() {
		showInviteModal = true;
	}

	function handleCloseInviteModal() {
		showInviteModal = false;
	}

	// Check if user can invite
	let canInviteUsers = $derived(
		!isReadOnly && currentUser
			? (permissions.getMetadata(currentUser.permissions)?.grantable_user_permissions?.length ??
					0) > 0
			: false
	);

	async function handleBulkDelete(ids: string[]) {
		if (confirm(common_confirmBulkDelete({ count: ids.length, entity: common_users() }))) {
			await bulkDeleteUsersMutation.mutateAsync(ids);
		}
	}

	function handleEditUser(user: User) {
		editingUser = user;
		showEditModal = true;
	}

	function handleCloseEditModal() {
		showEditModal = false;
		editingUser = null;
	}

	// CSV export handler (exports users only, not invites)
	async function handleCsvExport() {
		await downloadCsv('User', {});
	}

	// Only define fields for users (invites won't be filtered/sorted)
	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);

	/**
	 * The sites a user can reach.
	 *
	 * Admins and owners reach every site, which is a different statement from
	 * being assigned all of them — so it reads as one "All" chip rather than a
	 * list that would go stale the moment a site is added.
	 */
	function userSiteItems(item: UserOrInvite): LabelledCardFieldItem[] {
		if (!isUser(item)) return [];
		const user = item.data;

		if (user.permissions === 'Admin' || user.permissions === 'Owner') {
			return [{ id: 'all', label: common_all(), color: entities.getColorHelper('Site').color }];
		}

		return siteItems(user.site_ids ?? [], sitesData);
	}

	const revokeInviteMutation = useRevokeInviteMutation();

	/** Whether the current user is allowed to grant — and so revoke — this invite. */
	function canRevoke(invite: OrganizationInvite): boolean {
		return currentUser
			? (permissions
					.getMetadata(currentUser.permissions)
					?.grantable_user_permissions?.includes(invite.permissions) ?? false)
			: false;
	}

	function handleRevokeInvite(invite: OrganizationInvite) {
		if (confirm(invites_confirmRevoke())) {
			revokeInviteMutation.mutate(invite.id);
		}
	}

	const deleteUserMutation = useDeleteUserMutation();

	function handleDeleteUser(user: User) {
		if (confirm(users_confirmDeleteUser())) {
			deleteUserMutation.mutate(user.id);
		}
	}

	/** Whether the current user may manage the given user's account. */
	function canManageUser(user: User): boolean {
		return currentUser
			? (permissions
					.getMetadata(currentUser.permissions)
					?.grantable_user_permissions?.includes(user.permissions) ?? false)
			: false;
	}

	/** Row actions. A user is edited or deleted; an invite is revoked. */
	function userActions(item: UserOrInvite): CardAction[] {
		if (isUser(item)) {
			const actions: CardAction[] = [
				{ label: common_edit(), icon: Edit, onClick: () => handleEditUser(item.data) }
			];
			// You cannot delete yourself, and you cannot delete someone whose role
			// you could not have granted in the first place.
			if (item.data.id !== currentUser?.id && canManageUser(item.data)) {
				actions.push({
					label: common_delete(),
					icon: Trash2,
					class: 'btn-icon-danger',
					onClick: () => handleDeleteUser(item.data)
				});
			}
			return actions;
		}
		if (!canRevoke(item.data)) return [];
		return [
			{
				label: common_revoke(),
				icon: UserX,
				class: 'btn-icon-danger',
				onClick: () => handleRevokeInvite(item.data)
			}
		];
	}

	/**
	 * The list holds two shapes, so every field resolves for both. A field that
	 * only one variant has returns empty for the other.
	 */
	const tableDefaults: TableDefaults<string> = { sort: { field: 'email', direction: 'asc' } };

	const userFields: FieldConfig<UserOrInvite>[] = [
		{
			key: 'email',
			label: common_email(),
			type: 'string',
			searchable: true,
			sortable: true,
			getValue(item) {
				return isUser(item) ? item.data.email : item.data.send_to || invites_pendingInvite();
			},
			display: { primary: true, width: 220 }
		},
		{
			key: 'invite_status',
			label: common_status(),
			type: 'string',
			filterable: true,
			groupable: true,
			sortable: true,
			getValue: (item) =>
				!isUser(item)
					? invites_pendingInvite()
					: item.data.id === currentUser?.id
						? common_you()
						: common_user(),
			display: {
				getItems: (item) => {
					if (!isUser(item)) {
						return [{ id: 'pending', label: invites_pendingInvite(), color: 'Yellow' }];
					}
					return item.data.id === currentUser?.id
						? [{ id: 'you', label: common_you(), color: 'Yellow' }]
						: [];
				}
			}
		},
		{
			key: 'permissions',
			label: common_role(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			sortable: true,
			// Every role the backend defines, named the way the chip renders them.
			filterOptions: permissions.getItems().map((role) => permissions.getName(role.id)),
			getValue(item) {
				return permissions.getName(item.data.permissions) || item.data.permissions;
			},
			display: {
				getItems: (item) => {
					const role = item.data.permissions;
					return [
						{
							id: role,
							label: permissions.getName(role) || role,
							color: permissions.getColorHelper(role).color
						}
					];
				}
			}
		},
		{
			key: 'site_ids',
			label: common_sites(),
			type: 'array',
			searchable: true,
			// Users share sites, so this filters; as an array it neither sorts nor groups.
			filterable: true,
			getValue: (item) => userSiteItems(item).map((n) => n.label),
			display: { getItems: userSiteItems }
		},
		{
			key: 'oidc_provider',
			label: users_authMethod(),
			type: 'string',
			searchable: true,
			filterable: true,
			groupable: true,
			sortable: true,
			getValue(item) {
				if (!isUser(item)) return null;
				const { oidc_provider, has_password } = item.data;
				if (oidc_provider) {
					return has_password
						? users_authMethodOidcAndPassword({ provider: oidc_provider })
						: oidc_provider;
				}
				return has_password ? common_emailAndPassword() : common_none();
			}
		},
		{
			key: 'created_at',
			label: common_joined(),
			type: 'date',
			sortable: true,
			getValue: (item) => item.data.created_at
		},
		{
			key: 'invite_url',
			label: common_url(),
			type: 'string',
			searchable: true,
			getValue: (item) => (isUser(item) ? '' : formatInviteUrl(item.data)),
			display: { hiddenByDefault: true }
		},
		{
			key: 'invited_by',
			label: invites_createdBy(),
			type: 'string',
			searchable: true,
			sortable: true,
			groupable: true,
			filterable: true,
			getValue: (item) =>
				isUser(item)
					? null
					: usersData.find((u) => u.id == item.data.created_by)?.email ||
						common_unknownEntity({ entity: common_user() }),
			display: { hiddenByDefault: true }
		},
		{
			key: 'expires_at',
			label: common_expires(),
			type: 'date',
			sortable: true,
			getValue: (item) => (isUser(item) ? null : item.data.expires_at),
			display: { hiddenByDefault: true }
		},
		{
			key: 'updated_at',
			label: common_updated(),
			type: 'date',
			sortable: true,
			getValue: (item) => item.data.updated_at,
			display: { hiddenByDefault: true }
		},
		// Account facts an invite does not have yet, so its cells stay empty.
		{
			key: 'email_verified',
			label: users_emailVerified(),
			type: 'boolean',
			filterable: true,
			groupable: true,
			getValue: (item) => (isUser(item) ? (item.data.email_verified ?? null) : null),
			display: { hiddenByDefault: true }
		},
		{
			key: 'oidc_linked_at',
			label: users_providerLinked(),
			type: 'date',
			sortable: true,
			getValue: (item) => (isUser(item) ? (item.data.oidc_linked_at ?? null) : null),
			display: { hiddenByDefault: true }
		},
		{
			key: 'terms_accepted_at',
			label: users_termsAccepted(),
			type: 'date',
			sortable: true,
			getValue: (item) => (isUser(item) ? (item.data.terms_accepted_at ?? null) : null),
			display: { hiddenByDefault: true }
		}
	];
</script>

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	<div class="flex items-center gap-3">
		{#if seatLimit !== null && !canBuyMoreSeats}
			<span class="text-sm {isAtSeatLimit ? 'text-amber-400' : 'text-tertiary'}">
				{userCount} / {seatLimit}
			</span>
		{/if}
		{#if canInviteUsers}
			{#if isAtSeatLimit}
				<UpgradeButton feature="seats" surface="users_tab" gate_type="limit_hit" />
			{:else if currentUser && !currentUser.email_verified}
				<span data-tooltip={users_verifyEmailToInvite()} use:tooltip>
					<button class="btn-primary toolbar-control flex items-center opacity-50" disabled>
						<UserPlus class="mr-2 h-5 w-5" />
						{users_inviteUser()}
					</button>
				</span>
			{:else}
				<button class="btn-primary toolbar-control flex items-center" onclick={handleCreateInvite}>
					<UserPlus class="mr-2 h-5 w-5" />
					{users_inviteUser()}
				</button>
			{/if}
		{/if}
	</div>
{/snippet}

<div class="space-y-6">
	<!-- Loading state -->
	{#if isLoading}
		<Loading />
	{:else if combinedItems.length === 0}
		<!-- Empty state -->
		<EmptyState title={users_noUsersFound()} subtitle={users_noUsersSubtitle()} />
	{:else}
		<DataControls
			title={common_users()}
			subtitle={users_subtitle()}
			{toolbarActions}
			items={combinedItems}
			fields={userFields}
			storageKey="scanopy-users-table-state"
			defaults={tableDefaults}
			onBulkDelete={handleBulkDelete}
			getItemId={(item) => item.id}
			getActions={userActions}
			onCsvExport={handleCsvExport}
		></DataControls>
	{/if}
</div>

<InviteModal name="invite-user" isOpen={showInviteModal} onClose={handleCloseInviteModal} />
<UserEditModal
	name="user-editor"
	isOpen={showEditModal}
	user={editingUser}
	onClose={handleCloseEditModal}
/>
