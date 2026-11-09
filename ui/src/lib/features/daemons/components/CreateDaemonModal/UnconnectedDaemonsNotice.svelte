<script lang="ts">
	// Shown in place of the plan or verification banner when the wizard is refused and the org
	// holds a daemon that never connected. Each way forward is its own option: fix that daemon,
	// delete it and set up a new one here, or meet every requirement for a second daemon.
	import type { components } from '$lib/api/schema';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { formatDate } from '$lib/shared/utils/formatting';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import {
		useEmailVerificationPending,
		useResendVerificationMutation
	} from '$lib/features/auth/queries';
	import { useDeleteDaemonMutation } from '../../queries';
	import { DAEMON_STATUS_DOCS_URL } from '../../utils';
	import { daemonHealths } from '$lib/shared/stores/metadata';
	import {
		billing_lapsedBannerCta,
		common_confirmDeleteName,
		common_sending,
		daemons_unconnectedAddAnotherBody,
		daemons_unconnectedAddAnotherTitle,
		daemons_unconnectedBody,
		daemons_unconnectedCreated,
		daemons_unconnectedDelete,
		daemons_unconnectedFixBody,
		daemons_unconnectedFixTitle,
		daemons_unconnectedStartOverBody,
		daemons_unconnectedStartOverTitle,
		daemons_unconnectedStepVerify,
		daemons_unconnectedTitle,
		daemons_unconnectedTitleMany,
		daemons_unconnectedTroubleshoot,
		verifyEmail_resend
	} from '$lib/paraglide/messages';

	type ProvisionEligibility = components['schemas']['ProvisionEligibility'];
	type UnconnectedDaemon = components['schemas']['UnconnectedDaemon'];

	let { eligibility, onClose }: { eligibility: ProvisionEligibility; onClose: () => void } =
		$props();

	let daemons = $derived(eligibility.unconnected_daemons);

	const deleteDaemonMutation = useDeleteDaemonMutation();
	const verification = useEmailVerificationPending();
	const resendMutation = useResendVerificationMutation();

	const awaitingTag = daemonHealths.getTag('awaiting_connection');
	const troubleshootUrl = `${DAEMON_STATUS_DOCS_URL}#awaiting-connection`;

	function handleDelete(daemon: UnconnectedDaemon) {
		if (confirm(common_confirmDeleteName({ name: daemon.name }))) {
			// The delete re-reads eligibility, so this notice goes away once nothing refuses.
			deleteDaemonMutation.mutate(daemon.id);
		}
	}

	function handleResend() {
		if (verification.email) resendMutation.mutate({ email: verification.email });
	}

	function handleChoosePlan() {
		triggerUpgrade({
			source: 'create_daemon_plan_required',
			surface: 'sidebar',
			gate_type: 'limit_hit',
			beforeModal: () => onClose()
		});
	}
</script>

<div class="card card-static space-y-4">
	<div class="space-y-2">
		<h3 class="text-sm font-semibold text-primary">
			{daemons.length === 1
				? daemons_unconnectedTitle({ name: daemons[0].name })
				: daemons_unconnectedTitleMany({ count: daemons.length })}
		</h3>
		<ul class="space-y-1">
			{#each daemons as daemon (daemon.id)}
				<li class="flex flex-wrap items-center gap-2 text-sm">
					<span class="font-medium text-primary">{daemon.name}</span>
					<Tag {...awaitingTag} />
					<span class="text-tertiary">
						{daemons_unconnectedCreated({ date: formatDate(daemon.created_at) })}
					</span>
				</li>
			{/each}
		</ul>
		<p class="text-sm text-secondary">{daemons_unconnectedBody()}</p>
	</div>

	<div class="grid gap-3 sm:grid-cols-3">
		<div class="card flex flex-col gap-2">
			<h4 class="text-sm font-semibold text-primary">{daemons_unconnectedFixTitle()}</h4>
			<p class="flex-1 text-sm text-secondary">{daemons_unconnectedFixBody()}</p>
			{#each daemons as daemon (daemon.id)}
				<a class="btn-secondary" href={troubleshootUrl} target="_blank" rel="noopener noreferrer">
					{daemons_unconnectedTroubleshoot({ name: daemon.name })}
				</a>
			{/each}
		</div>

		<div class="card flex flex-col gap-2">
			<h4 class="text-sm font-semibold text-primary">{daemons_unconnectedStartOverTitle()}</h4>
			<p class="flex-1 text-sm text-secondary">{daemons_unconnectedStartOverBody()}</p>
			{#each daemons as daemon (daemon.id)}
				<button
					type="button"
					class="btn-danger"
					disabled={deleteDaemonMutation.isPending}
					onclick={() => handleDelete(daemon)}
				>
					{daemons_unconnectedDelete({ name: daemon.name })}
				</button>
			{/each}
		</div>

		<div class="card flex flex-col gap-2">
			<h4 class="text-sm font-semibold text-primary">{daemons_unconnectedAddAnotherTitle()}</h4>
			<p class="text-sm text-secondary">{daemons_unconnectedAddAnotherBody()}</p>
			<!-- Verification first: it completes in place, and the eligibility query refetches on
			     focus while it is pending, so the user comes back to only the plan step. Checkout
			     closes the wizard, so it goes last. -->
			<ol class="flex-1 list-decimal space-y-2 pl-5 text-sm text-secondary">
				{#if eligibility.email_verification_required}
					<li>
						{daemons_unconnectedStepVerify()}
						<button
							type="button"
							class="text-link hover:underline disabled:opacity-50"
							onclick={handleResend}
							disabled={resendMutation.isPending || !verification.email}
						>
							{resendMutation.isPending ? common_sending() : verifyEmail_resend()}
						</button>
					</li>
				{/if}
				{#if eligibility.plan_required}
					<li>
						<button type="button" class="text-link hover:underline" onclick={handleChoosePlan}>
							{billing_lapsedBannerCta()}
						</button>
					</li>
				{/if}
			</ol>
		</div>
	</div>
</div>
