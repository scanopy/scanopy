<script lang="ts">
	import { ChevronRight, CircleCheck, Lock, X } from '@lucide/svelte';
	import { useRecordSetupItemMutation } from '$lib/features/organizations/queries';
	import type { components } from '$lib/api/schema';
	import DiscoveryEstimation from '$lib/features/discovery/components/DiscoveryEstimation.svelte';
	import SessionProgress from '$lib/features/discovery/components/cards/SessionProgress.svelte';
	import { isFullScan } from '$lib/features/discovery/utils/fullScan';
	import { setupItems } from '$lib/shared/stores/metadata';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { common_dismiss, gettingStarted_emailNotification } from '$lib/paraglide/messages';
	import type { SetupItemType, SetupListEntry } from '../setup-list';
	import { setupRowView, type SetupLookups } from '../setup-row-view';
	import SetupItemSummary from './SetupItemSummary.svelte';

	type DiscoveryUpdatePayload = components['schemas']['DiscoveryUpdatePayload'];

	let {
		entry,
		setup,
		lookups,
		session = null,
		hasEmail = false,
		onAnswer,
		onTroubleshoot
	}: {
		entry: SetupListEntry;
		/** The org's done Setup item types. */
		setup: SetupItemType[];
		lookups: SetupLookups;
		/** The running first scan, for a RunFirstScan row. */
		session?: DiscoveryUpdatePayload | null;
		hasEmail?: boolean;
		/** Opens the question a question item asks. */
		onAnswer: (itemType: 'AnswerReferralSource' | 'AnswerProfile') => void;
		/** Opens the daemon troubleshooting help. */
		onTroubleshoot: () => void;
	} = $props();

	let item = $derived(entry.item);
	let done = $derived(entry.state === 'Done');
	/** A step waiting on an earlier one: shown disabled, with nothing to click. */
	let locked = $derived(entry.state === 'Locked');
	/** A question the user can dismiss unanswered. */
	let dismissable = $derived(item.type === 'AnswerReferralSource' || item.type === 'AnswerProfile');
	const recordSetupItem = useRecordSetupItemMutation();
	/** A first-scan row whose scan is running shows its progress, with nothing to click. */
	let running = $derived(!done && !locked && item.type === 'RunFirstScan' && session !== null);
	let Icon = $derived(setupItems.getIconComponent(item.type));
	let iconColors = $derived(setupItems.getColorHelper(item.type));

	let view = $derived(setupRowView(item, { setup, lookups, onAnswer, onTroubleshoot }));

	function runFix() {
		trackEvent('setup_item_action_clicked', { item_type: item.type });
		view.fix.run();
	}
</script>

{#snippet body()}
	<!-- The icon tile the app's other interactive cards use (SupportTab, ConfirmEntityModal). -->
	<div
		class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {done || locked
			? ''
			: iconColors.bg}"
	>
		{#if done}
			<CircleCheck class="h-5 w-5 text-success" />
		{:else if locked}
			<Lock class="h-5 w-5 text-tertiary" />
		{:else}
			<Icon class="h-5 w-5 {iconColors.icon}" />
		{/if}
	</div>
	<div class="min-w-0 flex-1 space-y-1">
		<SetupItemSummary {item} {view} {done} />
		{#if running && session}
			<div class="space-y-1 pt-1">
				<DiscoveryEstimation
					phase={session.phase}
					hosts_discovered={session.hosts_discovered}
					estimated_remaining_secs={session.estimated_remaining_secs}
					is_full_scan={isFullScan(session.discovery_type)}
				/>
				<SessionProgress
					session_id={session.session_id}
					phase={session.phase}
					progress={session.progress}
				/>
				{#if hasEmail}
					<p class="text-xs text-secondary">{gettingStarted_emailNotification()}</p>
				{/if}
			</div>
		{/if}
	</div>
{/snippet}

<!-- An open row is an interactive card (`card` without `card-static` rings on hover, as the
     app's other clickable cards do). A transparent button covers the card and runs the fix; the
     content sits over it with pointer events off, except the entity chips, which keep their own
     tooltip and click. The label on the right says what the card click does. A done row, a locked
     row, and a first-scan row whose scan is running, is a static card that only reports. A question
     row also has an X that dismisses it unanswered. -->
<li>
	{#if done || locked || running}
		<div
			class="card card-static flex items-center gap-3 p-4 {done
				? 'opacity-70'
				: locked
					? 'opacity-50'
					: ''}"
		>
			{@render body()}
		</div>
	{:else}
		<div
			class="card relative flex cursor-pointer items-center gap-3 p-4 has-focus-visible:ring-2 has-focus-visible:ring-blue-500"
		>
			<button
				type="button"
				class="absolute inset-0 cursor-pointer rounded-2xl focus-visible:outline-none"
				aria-label="{view.title}: {view.fix.label}"
				onclick={runFix}
			></button>
			<div class="pointer-events-none relative flex min-w-0 flex-1 items-center gap-3">
				{@render body()}
				<span class="flex shrink-0 items-center gap-1 text-sm text-secondary">
					{view.fix.label}
					<ChevronRight size={16} />
				</span>
				{#if dismissable}
					<button
						type="button"
						class="btn-icon pointer-events-auto relative z-10 shrink-0 rounded-lg p-1"
						aria-label={common_dismiss()}
						title={common_dismiss()}
						onclick={(e) => {
							e.stopPropagation();
							recordSetupItem.mutate(item.type);
						}}
					>
						<X size={16} />
					</button>
				{/if}
			</div>
		</div>
	{/if}
</li>
