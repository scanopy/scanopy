<script lang="ts">
	import GenericModal from './GenericModal.svelte';
	import TroubleshootingChecklist from '#lib/features/daemons/components/TroubleshootingChecklist.svelte';
	import {
		useRetryDaemonConnectionMutation,
		useTestReachabilityMutation
	} from '#lib/features/daemons/queries.ts';
	import type { Daemon } from '#lib/features/daemons/types/base.ts';
	import { useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import { daemons_troubleshootingTitle } from '#lib/paraglide/messages.js';

	let {
		daemon,
		isOpen = false,
		onClose
	}: {
		/** The daemon that isn't connecting. Its mode, OS and URL pick the checks shown. */
		daemon: Daemon | null | undefined;
		isOpen?: boolean;
		onClose: () => void;
	} = $props();

	const configQuery = useConfigQuery();
	const healthCheckMutation = useTestReachabilityMutation();
	const retryConnectionMutation = useRetryDaemonConnectionMutation();

	let healthResult = $state<{ reachable: boolean; health?: boolean; error?: string } | null>(null);
	let isCheckingHealth = $state(false);

	// A result belongs to the daemon it was taken for.
	$effect(() => {
		void daemon?.id;
		healthResult = null;
	});

	async function handleHealthCheck() {
		if (!daemon?.url) return;
		const id = daemon.id;
		isCheckingHealth = true;
		try {
			const result = await healthCheckMutation.mutateAsync({
				url: daemon.url,
				check_health: true
			});
			healthResult = {
				reachable: result.reachable,
				health: result.health ?? undefined,
				error: result.error ?? undefined
			};
			// Reachable and healthy: clear the unreachable flag so the server polls it again.
			if (result.reachable && result.health) retryConnectionMutation.mutate(id);
		} catch {
			healthResult = { reachable: false };
		} finally {
			isCheckingHealth = false;
		}
	}
</script>

<GenericModal
	title={daemons_troubleshootingTitle()}
	isOpen={isOpen && !!daemon}
	{onClose}
	size="xl"
>
	{#if daemon}
		<div class="min-h-0 flex-1 overflow-auto p-6">
			<TroubleshootingChecklist
				mode={daemon.mode}
				serverUrl={configQuery.data?.public_url ?? ''}
				daemonUrl={daemon.url}
				daemonName={daemon.name}
				selectedOS={daemon.os ?? 'linux'}
				onHealthCheck={handleHealthCheck}
				{isCheckingHealth}
				{healthResult}
			/>
		</div>
	{/if}
</GenericModal>
