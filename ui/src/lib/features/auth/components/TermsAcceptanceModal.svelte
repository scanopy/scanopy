<script lang="ts">
	import { FileText, LogOut } from '@lucide/svelte';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import MessageWithLinks from '#lib/shared/components/data/MessageWithLinks.svelte';
	import { slot } from '#lib/shared/components/data/MessageWithSlots.svelte';
	import { PRIVACY_URL, TERMS_URL } from '#lib/shared/utils/site-links.ts';
	import { useAcceptTermsMutation, useLogoutMutation } from '#lib/features/auth/queries.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		auth_termsAccept,
		auth_termsUpdatedBody,
		auth_privacyLinkText,
		auth_termsOfServiceLinkText,
		auth_termsUpdatedTitle,
		common_logout
	} from '#lib/paraglide/messages.js';

	// No onClose: the server refuses this user's requests until they accept, so
	// the only ways out are accepting and logging out.
	let { isOpen }: { isOpen: boolean } = $props();

	const acceptTermsMutation = useAcceptTermsMutation();
	const logoutMutation = useLogoutMutation();
	let busy = $derived(acceptTermsMutation.isPending || logoutMutation.isPending);

	let actions = $derived<ModalAction[]>([
		{
			role: 'secondary',
			label: common_logout(),
			icon: LogOut,
			// A full load, as the Account tab does: AppShell stays mounted across a
			// client-side navigation and would keep the old session's queries.
			onClick: () =>
				logoutMutation.mutateAsync().then(
					() => window.location.reload(),
					() => {}
				),
			disabled: busy
		},
		{
			role: 'primary',
			label: auth_termsAccept(),
			onClick: () => acceptTermsMutation.mutate(),
			disabled: busy
		}
	]);
</script>

<GenericModal
	title={auth_termsUpdatedTitle()}
	{isOpen}
	size="md"
	preventCloseOnClickOutside={true}
	showCloseButton={false}
	{actions}
>
	{#snippet headerIcon()}
		<FileText class="h-5 w-5 text-blue-400" />
	{/snippet}
	<div class="px-6 py-4">
		<p class="text-sm text-secondary">
			<MessageWithLinks
				text={auth_termsUpdatedBody({ terms: slot('terms'), privacy: slot('privacy') })}
				links={{
					terms: { href: TERMS_URL, label: auth_termsOfServiceLinkText() },
					privacy: { href: PRIVACY_URL, label: auth_privacyLinkText() }
				}}
			/>
		</p>
	</div>
</GenericModal>
