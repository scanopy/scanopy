<script lang="ts">
	import { Copy, ExternalLink, RefreshCw } from 'lucide-svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import ConfirmationDialog from '$lib/shared/components/feedback/ConfirmationDialog.svelte';
	import {
		useCreateLicenseKeyMutation,
		useRegenerateLicenseKeyMutation
	} from '$lib/features/billing/queries';
	import type { Organization } from '$lib/features/organizations/types';
	import type { components } from '$lib/api/schema';
	import { billingPlans, licenseKeyTypes } from '$lib/shared/stores/metadata';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { copyViaSelection } from '$lib/shared/utils/clipboard';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import {
		common_copied,
		common_failedToCopy,
		common_license,
		common_never,
		common_tier,
		settings_billing_changePlan,
		settings_billing_license_bookDemo,
		settings_billing_license_copyOfflineKey,
		settings_billing_license_copyOnlineKey,
		settings_billing_license_installDocs,
		settings_billing_license_lastCheckIn,
		settings_billing_license_minServerVersion,
		settings_billing_license_offlineKeyUpsell,
		settings_billing_license_paidThrough,
		settings_billing_license_regenerate,
		settings_billing_license_regenerateConfirm,
		settings_billing_license_regenerateTitle,
		settings_billing_license_regenerated
	} from '$lib/paraglide/messages';

	type LicenseKeyType = components['schemas']['LicenseKeyType'];

	let {
		org,
		onChangePlan
	}: {
		org: Organization;
		onChangePlan: () => void;
	} = $props();

	const createKeyMutation = useCreateLicenseKeyMutation();
	const regenerateMutation = useRegenerateLicenseKeyMutation();

	let planType = $derived(org.plan?.type ?? null);
	let offlineKeysIncluded = $derived(
		billingPlans.getMetadata(planType).features?.air_gapped_deployment === true
	);
	const onlineMinServerVersion = licenseKeyTypes.getMetadata('Online').min_server_version;

	let paidThrough = $derived(
		org.license_paid_through
			? new Date(org.license_paid_through).toLocaleDateString(undefined, {
					month: 'long',
					day: 'numeric',
					year: 'numeric'
				})
			: null
	);
	let lastCheckIn = $derived(
		org.license_last_checked_in_at
			? formatTimestamp(org.license_last_checked_in_at)
			: common_never()
	);

	let copyingKeyType = $state<LicenseKeyType | null>(null);
	let showRegenerateConfirm = $state(false);

	async function copyKey(keyType: LicenseKeyType) {
		copyingKeyType = keyType;
		let mintFailed = false;
		const key = createKeyMutation.mutateAsync(keyType).catch((error: unknown) => {
			mintFailed = true;
			throw error;
		});
		try {
			await writeClipboard(key);
			pushSuccess(common_copied());
			trackEvent('license_key_copied', { key_type: keyType });
		} catch (error) {
			// The API client already toasted a failed mint.
			if (!mintFailed) pushWarning(common_failedToCopy({ error: String(error) }));
		} finally {
			copyingKeyType = null;
		}
	}

	/**
	 * The key exists only once the mint request returns, and by then the click's user activation
	 * can have lapsed: Safari refuses a `writeText` issued after an await. A `ClipboardItem` built
	 * from the pending key claims the clipboard inside the click and fills it when the key
	 * arrives. Where that is unsupported, `writeText` runs after the mint; plain-HTTP self-hosts,
	 * which have no `navigator.clipboard`, use a selection copy.
	 */
	async function writeClipboard(key: Promise<string>): Promise<void> {
		if (window.isSecureContext && navigator.clipboard) {
			if (typeof ClipboardItem !== 'undefined') {
				try {
					const blob = key.then((text) => new Blob([text], { type: 'text/plain' }));
					await navigator.clipboard.write([new ClipboardItem({ 'text/plain': blob })]);
					return;
				} catch {
					// Fall back to writeText below.
				}
			}
			await navigator.clipboard.writeText(await key);
			return;
		}
		if (!copyViaSelection(await key)) throw new Error('the browser refused the copy');
	}

	async function handleRegenerate() {
		showRegenerateConfirm = false;
		try {
			await regenerateMutation.mutateAsync();
			pushSuccess(settings_billing_license_regenerated());
			trackEvent('license_key_regenerated');
		} catch {
			// The API client toasts the error.
		}
	}
</script>

<InfoCard title={common_license()}>
	<div class="space-y-4">
		<dl class="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
			<dt class="text-secondary">{common_tier()}</dt>
			<dd class="text-primary font-medium">{billingPlans.getName(planType)}</dd>
			{#if paidThrough}
				<dt class="text-secondary">{settings_billing_license_paidThrough()}</dt>
				<dd class="text-primary">{paidThrough}</dd>
			{/if}
			<dt class="text-secondary">{settings_billing_license_lastCheckIn()}</dt>
			<dd class="text-primary">{lastCheckIn}</dd>
		</dl>

		<div class="space-y-3 border-t pt-3" style="border-color: var(--color-border)">
			<div class="flex flex-wrap items-center gap-x-3 gap-y-1">
				<button
					type="button"
					class="btn-primary flex items-center gap-2"
					onclick={() => copyKey('Online')}
					disabled={copyingKeyType !== null}
				>
					<Copy class="h-4 w-4" />
					{settings_billing_license_copyOnlineKey()}
				</button>
				{#if onlineMinServerVersion}
					<p class="text-secondary text-xs">
						{settings_billing_license_minServerVersion({ version: onlineMinServerVersion })}
					</p>
				{/if}
			</div>

			{#if offlineKeysIncluded}
				<button
					type="button"
					class="btn-secondary flex items-center gap-2"
					onclick={() => copyKey('Offline')}
					disabled={copyingKeyType !== null}
				>
					<Copy class="h-4 w-4" />
					{settings_billing_license_copyOfflineKey()}
				</button>
			{:else}
				<p class="text-secondary text-sm">
					{settings_billing_license_offlineKeyUpsell()}
					<button type="button" onclick={onChangePlan} class="text-link hover:underline">
						{settings_billing_changePlan()}
					</button>
				</p>
			{/if}

			<button
				type="button"
				class="btn-secondary flex items-center gap-2"
				onclick={() => (showRegenerateConfirm = true)}
				disabled={regenerateMutation.isPending}
			>
				<RefreshCw class="h-4 w-4" />
				{settings_billing_license_regenerate()}
			</button>
		</div>

		<div
			class="flex flex-wrap gap-x-4 gap-y-1 border-t pt-3 text-sm"
			style="border-color: var(--color-border)"
		>
			<a
				href="https://scanopy.net/docs/self-hosted-server/server-installation/"
				target="_blank"
				rel="noopener noreferrer"
				class="text-link inline-flex items-center gap-1 hover:underline"
			>
				{settings_billing_license_installDocs()}
				<ExternalLink class="h-3 w-3" />
			</a>
			<a
				href="https://cal.com/mferrandiz/scanopy-demo"
				target="_blank"
				rel="noopener noreferrer"
				class="text-link inline-flex items-center gap-1 hover:underline"
			>
				{settings_billing_license_bookDemo()}
				<ExternalLink class="h-3 w-3" />
			</a>
		</div>
	</div>
</InfoCard>

<ConfirmationDialog
	isOpen={showRegenerateConfirm}
	title={settings_billing_license_regenerateTitle()}
	message={settings_billing_license_regenerateConfirm()}
	confirmLabel={settings_billing_license_regenerate()}
	variant="danger"
	onConfirm={handleRegenerate}
	onCancel={() => (showRegenerateConfirm = false)}
	onClose={() => (showRegenerateConfirm = false)}
/>
