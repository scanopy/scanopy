<script lang="ts">
	import { Trash2 } from '@lucide/svelte';
	import InfoCard from '#lib/shared/components/data/InfoCard.svelte';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import ConfirmationDialog from '#lib/shared/components/feedback/ConfirmationDialog.svelte';
	import {
		useDeleteLicenseInstanceMutation,
		useLicenseInstancesQuery
	} from '#lib/features/billing/queries.ts';
	import { formatTimestamp } from '#lib/shared/utils/formatting.ts';
	import {
		common_delete,
		common_inactive,
		common_ipAddress,
		common_nonProduction,
		common_production,
		common_readOnly,
		common_servers,
		settings_billing_license_lastCheckIn,
		settings_billing_license_noServers,
		settings_billing_license_noServersLinkText,
		settings_billing_license_nonProductionOverLimit,
		settings_billing_license_olderServers,
		settings_billing_license_overLimit,
		settings_billing_license_serverDeleteConfirm,
		settings_billing_license_serverDeleteTitle,
		settings_billing_license_serversHint
	} from '#lib/paraglide/messages.js';

	let { enabled }: { enabled: boolean } = $props();

	const instancesQuery = useLicenseInstancesQuery(() => enabled);
	const deleteMutation = useDeleteLicenseInstanceMutation();

	let instances = $derived(instancesQuery.data?.instances ?? []);
	let usage = $derived(instancesQuery.data?.usage ?? null);
	let pendingDelete = $state<string | null>(null);

	async function handleDelete() {
		const id = pendingDelete;
		pendingDelete = null;
		if (!id) return;
		try {
			await deleteMutation.mutateAsync(id);
		} catch {
			// The API client toasts the error.
		}
	}
</script>

<InfoCard title={common_servers()}>
	<div class="space-y-3">
		<p class="text-sm text-secondary">{settings_billing_license_serversHint()}</p>
		{#if usage && usage.limit != null && usage.active > usage.limit}
			<InlineWarning
				title={settings_billing_license_overLimit({ active: usage.active, limit: usage.limit })}
			/>
		{/if}
		{#if usage && usage.non_production_limit != null && usage.non_production_active > usage.non_production_limit}
			<InlineWarning
				title={settings_billing_license_nonProductionOverLimit({
					active: usage.non_production_active,
					limit: usage.non_production_limit
				})}
			/>
		{/if}
		{#if instancesQuery.isSuccess && instances.length === 0}
			<DocsHint
				text={settings_billing_license_noServers()}
				href="https://scanopy.net/docs/self-hosted-server/troubleshooting/?edition=commercial"
				linkText={settings_billing_license_noServersLinkText()}
			/>
		{/if}
		{#each instances as { instance, active, refused } (instance.id)}
			<InfoCard variant="compact">
				<div class="flex items-center justify-between gap-4">
					<div class="min-w-0 {active ? '' : 'opacity-60'}">
						<div class="flex min-w-0 items-center gap-2">
							<p class="truncate text-sm font-medium text-primary">
								{#if instance.instance_id}
									<span class="font-mono">{instance.instance_id}</span>
								{:else}
									{settings_billing_license_olderServers()}
								{/if}
							</p>
							<!-- Servers too old to declare a type carry no tag. -->
							{#if instance.non_production === true}
								<Tag label={common_nonProduction()} color="Gray" />
							{:else if instance.non_production === false}
								<Tag label={common_production()} color="Blue" />
							{/if}
							{#if !active}
								<span class="text-xs text-secondary">{common_inactive()}</span>
							{:else if refused}
								<Tag label={common_readOnly()} color="Red" />
							{/if}
						</div>
						<p class="text-xs text-secondary">
							{#if instance.last_seen_ip}
								{common_ipAddress()}: <span class="font-mono">{instance.last_seen_ip}</span> ·
							{/if}
							{#if instance.server_version}
								v{instance.server_version} ·
							{/if}
							{settings_billing_license_lastCheckIn()}: {formatTimestamp(instance.last_seen_at)}
						</p>
					</div>
					<button
						type="button"
						class="btn-secondary flex shrink-0 items-center gap-2"
						onclick={() => (pendingDelete = instance.id)}
						disabled={deleteMutation.isPending}
					>
						<Trash2 class="h-4 w-4" />
						{common_delete()}
					</button>
				</div>
			</InfoCard>
		{/each}
	</div>
</InfoCard>

<ConfirmationDialog
	isOpen={pendingDelete != null}
	title={settings_billing_license_serverDeleteTitle()}
	message={settings_billing_license_serverDeleteConfirm()}
	confirmLabel={common_delete()}
	variant="danger"
	onConfirm={handleDelete}
	onCancel={() => (pendingDelete = null)}
	onClose={() => (pendingDelete = null)}
/>
