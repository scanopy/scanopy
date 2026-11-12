<script lang="ts">
	import { collectionLookup } from '#lib/shared/entity-fields/index.ts';
	import CreateButton from '#lib/shared/components/data/controls/CreateButton.svelte';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import { userColumns } from './UserTab.columns';
	import { Edit, UserX, Trash2 } from '@lucide/svelte';
	import { disabledWhen, type RowAction } from '#lib/shared/components/data/types.ts';
	import Loading from '#lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '#lib/shared/components/layout/LoadFailedState.svelte';
	import { loadFailure } from '#lib/shared/utils/load-failure.ts';
	import EmptyState from '#lib/shared/components/layout/EmptyState.svelte';
	import DataControls from '#lib/shared/components/data/DataControls.svelte';
	import type { TableDefaults } from '#lib/shared/components/data/types.ts';
	import {
		formatInviteUrl,
		useBulkRevokeInvitesMutation,
		useRevokeInviteMutation
	} from '#lib/features/organizations/queries.ts';
	import { isUser, type User, type UserOrInvite } from '../types';
	import type { OrganizationInvite } from '#lib/features/organizations/types.ts';
	import InviteModal from './InviteModal.svelte';
	import { metadata, permissions } from '#lib/shared/stores/metadata.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import UserEditModal from './UserEditModal.svelte';
	import { useCurrentUserQuery } from '#lib/features/auth/queries.ts';
	import {
		useMembersQuery,
		useMembersListQuery,
		useBulkDeleteUsersMutation,
		useDeleteUserMutation
	} from '../queries';
	import { listPageState } from '#lib/shared/components/data/listPage.svelte.ts';
	import type { components } from '#lib/api/schema';

	type MemberOrderField = components['schemas']['MemberOrderField'];
	import type { TabProps } from '#lib/shared/types.ts';
	import { downloadCsv } from '#lib/shared/utils/csvExport.ts';
	import { modalState, resolveModalDeepLink } from '#lib/shared/stores/modal-registry.ts';
	import {
		common_edit,
		common_confirmBulkDelete,
		common_delete,
		common_revoke,
		common_users,
		invites_confirmBulkRevoke,
		invites_confirmRevoke,
		users_confirmDeleteUser,
		users_inviteUser,
		users_noUsersFound,
		users_noUsersSubtitle,
		users_subtitle,
		invites_cantRevokeUngrantable,
		invites_cantRevokeUngrantableBulk,
		users_cantDeleteSelf,
		users_cantDeleteSelfBulk,
		users_cantDeleteUngrantable,
		users_cantDeleteUngrantableBulk
	} from '#lib/paraglide/messages.js';

	import { useCan } from '#lib/features/auth/access.ts';

	let { isActive = false }: TabProps = $props();
	const can = useCan();

	// Query
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let org = $derived(organizationQuery.data);
	let seatLimit = $derived(org?.plan?.included_seats ?? null);
	let canBuyMoreSeats = $derived(
		org?.plan?.seat_cents !== undefined && org?.plan?.seat_cents !== null
	);

	// Every member the caller may see, for the seat count, filter options and deep links; the table
	// reads its own page.
	const membersQuery = useMembersQuery();
	const page = listPageState(() => isActive);
	const membersPageQuery = useMembersListQuery(() => page.params());
	const bulkDeleteUsersMutation = useBulkDeleteUsersMutation();

	// Derived data
	let membersData = $derived(membersQuery.data ?? []);
	let usersData = $derived(membersData.filter(isUser).map((member) => member.data));
	let pageMembers = $derived(membersPageQuery.data?.items ?? []);
	let membersPagination = $derived(membersPageQuery.data?.pagination ?? null);
	let isLoading = $derived(
		membersQuery.isPending || (membersPageQuery.isPending && !membersPageQuery.data)
	);
	let pageLoad = $derived(loadFailure(organizationQuery, membersQuery, membersPageQuery));

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

	let userCount = $derived(usersData.length);
	let isAtSeatLimit = $derived(seatLimit !== null && userCount >= seatLimit && !canBuyMoreSeats);

	async function handleCreateInvite() {
		showInviteModal = true;
	}

	function handleCloseInviteModal() {
		showInviteModal = false;
	}

	// Check if user can invite
	let canInviteUsers = $derived(can('Member', 'Write'));

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

	const revokeInviteMutation = useRevokeInviteMutation();
	const bulkRevokeInvitesMutation = useBulkRevokeInvitesMutation();

	/**
	 * Whether the current user may revoke this invite: one they created, or one for a role they
	 * could grant. The server applies the same rule.
	 */
	function canRevoke(invite: OrganizationInvite): boolean {
		if (currentUser && invite.created_by === currentUser.id) return true;
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

	async function handleBulkRevoke(ids: string[]) {
		if (confirm(invites_confirmBulkRevoke({ count: ids.length }))) {
			await bulkRevokeInvitesMutation.mutateAsync(ids);
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

	/** Whether the current user can grant any role, and so manage users and invites at all. */
	let managesUsers = $derived(
		currentUser
			? (permissions.getMetadata(currentUser.permissions)?.grantable_user_permissions?.length ??
					0) > 0
			: false
	);

	/** Why the current user can't delete this user, if they can't. The server applies the same rule. */
	function deleteRefusal(user: User) {
		return (
			disabledWhen(
				user.id === currentUser?.id,
				users_cantDeleteSelf(),
				users_cantDeleteSelfBulk()
			) ??
			disabledWhen(
				!canManageUser(user),
				users_cantDeleteUngrantable(),
				users_cantDeleteUngrantableBulk()
			)
		);
	}

	/** Row actions. A user is edited or deleted; an invite is revoked. */
	function userActions(item: UserOrInvite): RowAction[] {
		if (isUser(item)) {
			const actions: RowAction[] = [
				{ label: common_edit(), icon: Edit, isOpen: true, onClick: () => handleEditUser(item.data) }
			];
			if (managesUsers) {
				actions.push({
					label: common_delete(),
					icon: Trash2,
					tone: 'danger',
					isDelete: true,
					onBulk: handleBulkDelete,
					onClick: () => handleDeleteUser(item.data),
					unavailable: deleteRefusal(item.data)
				});
			}
			return actions;
		}
		if (!managesUsers) return [];
		return [
			{
				label: common_revoke(),
				icon: UserX,
				tone: 'danger',
				onClick: () => handleRevokeInvite(item.data),
				onBulk: handleBulkRevoke,
				unavailable: disabledWhen(
					!canRevoke(item.data),
					invites_cantRevokeUngrantable(),
					invites_cantRevokeUngrantableBulk()
				)
			}
		];
	}

	const tableDefaults: TableDefaults<MemberOrderField> = {
		sort: { field: 'email', direction: 'asc' }
	};

	// The registry's user fields reach no related entity.
	const lookup = collectionLookup({});

	let userFields = $derived(
		userColumns({
			lookup: () => lookup,
			members: () => membersData,
			sites: () => sitesData,
			inviteUrl: formatInviteUrl
		})
	);
</script>

<!-- The page's own actions, last in the table toolbar beside the filter and columns. -->
{#snippet toolbarActions()}
	<div class="flex items-center gap-3">
		{#if seatLimit !== null && !canBuyMoreSeats}
			<span
				class="text-sm {isAtSeatLimit ? 'text-amber-700 dark:text-amber-400' : 'text-tertiary'}"
			>
				{userCount} / {seatLimit}
			</span>
		{/if}
		{#if canInviteUsers}
			<CreateButton
				onclick={handleCreateInvite}
				label={users_inviteUser()}
				upgrade={isAtSeatLimit
					? { feature: 'seats', surface: 'users_tab', gate_type: 'limit_hit' }
					: null}
			/>
		{/if}
	</div>
{/snippet}

<div class="space-y-6">
	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if isLoading}
		<Loading />
	{:else if page.isEmpty(membersPageQuery)}
		<!-- Empty state -->
		<EmptyState title={users_noUsersFound()} subtitle={users_noUsersSubtitle()} />
	{:else}
		<DataControls
			title={common_users()}
			subtitle={users_subtitle()}
			{toolbarActions}
			items={pageMembers}
			fields={userFields}
			storageKey="scanopy-users-table-state"
			defaults={tableDefaults}
			getItemId={(item) => item.id}
			serverPagination={membersPagination}
			onPageChange={page.onPageChange}
			onQueryChange={page.onQueryChange}
			serverQuery={{ stale: false, tags: false }}
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
