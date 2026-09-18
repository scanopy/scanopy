<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import {
		UserPlus,
		Copy,
		Check,
		Calendar,
		Link as LinkIcon,
		RotateCcw,
		Send
	} from 'lucide-svelte';
	import { pushSuccess, pushError } from '$lib/shared/stores/feedback';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import type { OrganizationInvite } from '$lib/features/organizations/types';
	import { formatInviteUrl, useCreateInviteMutation } from '$lib/features/organizations/queries';
	import type { UserOrgPermissions } from '../types';
	import { email } from '$lib/shared/components/forms/validators';
	import { metadata, entities } from '$lib/shared/stores/metadata';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		common_close,
		common_copied,
		common_copyLink,
		common_email,
		common_generating,
		common_sending,
		users_copyFailed,
		users_emailHelp,
		users_emailPlaceholder,
		users_expires,
		users_generateInviteLink,
		users_inviteCopied,
		users_inviteFailed,
		users_inviteGeneratedSuccess,
		users_inviteInstructions,
		users_inviteLink,
		users_inviteSentSuccess,
		users_inviteUser,
		users_networkAccessHelp,
		users_permissionsLevel,
		users_permissionsLevelHelp,
		users_sendInviteLink,
		users_sensitiveLink,
		users_sensitiveLinkWarning
	} from '$lib/paraglide/messages';

	// Shared components
	import PermissionSelect from '$lib/shared/components/api-keys/PermissionSelect.svelte';
	import NetworkAccessSelect from '$lib/shared/components/api-keys/NetworkAccessSelect.svelte';

	let {
		isOpen = $bindable(false),
		onClose,
		name = undefined
	}: { isOpen: boolean; onClose: () => void; name?: string } = $props();

	// Mutation for creating invite
	const createInviteMutation = useCreateInviteMutation();

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let enableEmail = $derived(configData?.has_email_service ?? false);

	// Force Svelte to track reactivity
	$effect(() => {
		void $metadata;
	});

	let copied = $state(false);
	let copyTimeoutId = $state<ReturnType<typeof setTimeout> | null>(null);
	let generatingInvite = $derived(createInviteMutation.isPending);
	let invite = $state<OrganizationInvite | null>(null);

	// Track selected network IDs for the invite
	let selectedNetworkIds = $state<string[]>([]);

	// Create form
	const form = createForm(() => ({
		defaultValues: {
			permissions: 'Viewer' as UserOrgPermissions,
			email: ''
		},
		onSubmit: async () => {
			// Not used - we handle submission with handleGenerateInvite
		}
	}));

	let permissionsValue = $derived(form.state.values.permissions);
	let emailValue = $derived(form.state.values.email);
	let emailValid = $derived(!emailValue || !email(emailValue));

	let usingEmail = $derived(enableEmail && emailValue && emailValid);
	let ctaText = $derived(usingEmail ? users_sendInviteLink() : users_generateInviteLink());
	let ctaLoadingText = $derived(usingEmail ? common_sending() : common_generating());
	let CtaIcon = $derived(usingEmail ? Send : RotateCcw);

	// Handle network selection changes
	function handleNetworkChange(networkIds: string[]) {
		selectedNetworkIds = networkIds;
	}

	// Reset form when modal opens
	function handleOpen() {
		form.reset({ permissions: 'Viewer', email: '' });
		selectedNetworkIds = [];
		invite = null;
	}

	function handleClose() {
		invite = null;
		onClose();
	}

	async function handleGenerateInvite() {
		try {
			// Read values directly from form state to ensure we get current values
			const currentPermissions = form.state.values.permissions;
			const currentEmail = form.state.values.email.trim();

			const result = await createInviteMutation.mutateAsync({
				permissions: currentPermissions,
				network_ids: selectedNetworkIds,
				email: currentEmail
			});
			invite = result;
			pushSuccess(currentEmail ? users_inviteSentSuccess() : users_inviteGeneratedSuccess());
		} catch (err) {
			const action = form.state.values.email ? 'send' : 'generate';
			pushError(users_inviteFailed({ action, error: String(err) }));
		}
	}

	const isSecureContext =
		window.isSecureContext ||
		window.location.hostname === 'localhost' ||
		window.location.hostname === '127.0.0.1';

	async function handleCopy() {
		if (!invite) return;

		try {
			await navigator.clipboard.writeText(formatInviteUrl(invite));
			copied = true;
			pushSuccess(users_inviteCopied());

			// Reset copied state after 2 seconds
			if (copyTimeoutId) {
				clearTimeout(copyTimeoutId);
			}
			copyTimeoutId = setTimeout(() => {
				copied = false;
			}, 2000);
		} catch (err) {
			pushError(users_copyFailed());
			console.error('Failed to copy:', err);
		}
	}

	// Cleanup timeout on component destroy
	$effect(() => {
		if (!isOpen && copyTimeoutId) {
			clearTimeout(copyTimeoutId);
			copyTimeoutId = null;
			copied = false;
		}
	});
</script>

<GenericModal
	{isOpen}
	title={users_inviteUser()}
	{name}
	size="xl"
	onClose={handleClose}
	onOpen={handleOpen}
	showCloseButton={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={UserPlus} color={entities.getColorHelper('User').color} />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<p class="text-secondary text-sm">
					{users_inviteInstructions()}
				</p>

				<!-- Permissions Selection -->
				<form.Field name="permissions">
					{#snippet children(field)}
						<PermissionSelect
							{field}
							label={users_permissionsLevel()}
							helpText={users_permissionsLevelHelp()}
							disabled={!!invite}
						/>
					{/snippet}
				</form.Field>

				<NetworkAccessSelect
					{selectedNetworkIds}
					onChange={handleNetworkChange}
					permissionLevel={permissionsValue}
					helpText={users_networkAccessHelp()}
				/>

				{#if enableEmail}
					<form.Field name="email" validators={{ onBlur: ({ value }) => email(value) }}>
						{#snippet children(field)}
							<TextInput
								label={common_email()}
								id="email"
								placeholder={users_emailPlaceholder()}
								helpText={users_emailHelp()}
								{field}
							/>
						{/snippet}
					</form.Field>
				{/if}

				<!-- Generate Invite Button (shown when no invite exists) -->
				{#if !invite}
					<button
						onclick={handleGenerateInvite}
						type="button"
						disabled={generatingInvite || !emailValid}
						class="btn-primary w-full"
					>
						<CtaIcon class="mr-2 h-4 w-4" />
						{generatingInvite ? ctaLoadingText : ctaText}
					</button>
				{/if}

				<!-- Invite URL Card (shown when invite exists) -->
				{#if invite}
					<div class="card card-static">
						<div class="space-y-3">
							<div class="flex items-center gap-2">
								<LinkIcon class="text-secondary h-4 w-4 flex-shrink-0" />
								<h3 class="text-primary text-sm font-semibold">{users_inviteLink()}</h3>
							</div>

							<!-- URL Display -->
							<div class="card">
								<code class="text-primary block break-all text-sm">{formatInviteUrl(invite)}</code>
							</div>

							<!-- Copy Button -->
							{#if isSecureContext}
								<button
									onclick={handleCopy}
									type="button"
									class="btn-primary w-full"
									disabled={copied}
								>
									{#if copied}
										<Check class="mr-2 h-4 w-4" />
										{common_copied()}
									{:else}
										<Copy class="mr-2 h-4 w-4" />
										{common_copyLink()}
									{/if}
								</button>
							{/if}
						</div>
					</div>

					<!-- Expiration Info -->
					<div class="card card-static">
						<div class="flex items-center gap-3">
							<div
								class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg bg-blue-500/10"
							>
								<Calendar class="h-5 w-5 text-blue-400" />
							</div>
							<div class="flex-1">
								<p class="text-primary text-sm font-medium">
									{users_expires({ timestamp: formatTimestamp(invite.expires_at) })}
								</p>
							</div>
						</div>
					</div>

					<InlineWarning
						title={users_sensitiveLink()}
						body={users_sensitiveLinkWarning({ permissions: permissionsValue })}
					/>
				{/if}
			</div>
		</div>

		<!-- Footer -->
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={handleClose} class="btn-secondary">
					{common_close()}
				</button>
			</div>
		</div>
	</div>
</GenericModal>
