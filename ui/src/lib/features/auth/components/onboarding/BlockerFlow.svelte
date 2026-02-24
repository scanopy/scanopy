<script lang="ts">
	import { type BlockerType, type UseCase, BLOCKERS, getBlockerOptions } from '../../types/base';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import CompatibilityChecker from './CompatibilityChecker.svelte';
	import FeedbackForm from './FeedbackForm.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import { ChevronDown, ChevronRight, ExternalLink } from 'lucide-svelte';
	import {
		auth_scanopyLogo,
		onboarding_continueSetup,
		onboarding_letUsHelp,
		onboarding_whatDoYouNeed
	} from '$lib/paraglide/messages';

	export let isOpen: boolean;
	export let useCase: UseCase;
	export let onResolved: () => void;
	export let onClose: () => void;

	// Track which blocker is expanded (null = none)
	let expandedBlocker: BlockerType | null = null;
	let expandedBlockers: BlockerType[] = [];

	// Get blocker options for the current use case
	$: blockerOptions = getBlockerOptions(useCase);

	function toggleBlocker(blocker: BlockerType) {
		if (expandedBlocker === blocker) {
			expandedBlocker = null;
		} else {
			expandedBlocker = blocker;
			expandedBlockers.push(blocker);
			onboardingStore.setCurrentBlocker(blocker);
			trackEvent('onboarding_blocker_selected', { blocker, useCase });
		}
	}

	function handleContinueSetup() {
		trackEvent('onboarding_blockers_resolved', {
			blockers: expandedBlockers,
			useCase
		});
		onboardingStore.setReadyToScan(true);
		onResolved();
	}
</script>

<GenericModal
	{isOpen}
	title={onboarding_whatDoYouNeed()}
	{onClose}
	size="lg"
	centerTitle={true}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-y-auto p-6">
			<div class="space-y-1">
				<p class="text-secondary mb-3 text-center text-sm">{onboarding_letUsHelp()}</p>

				{#each blockerOptions as blockerId (blockerId)}
					{@const blocker = BLOCKERS[blockerId]}
					{@const isExpanded = expandedBlocker === blockerId}

					<div class="card overflow-hidden p-1">
						<button
							type="button"
							class="flex w-full items-center justify-between gap-3 px-3 py-2 text-left transition-colors hover:bg-gray-800"
							on:click={() => toggleBlocker(blockerId)}
						>
							<div class="flex items-center gap-3">
								<svelte:component this={blocker.Icon} class="text-tertiary h-4 w-4 flex-shrink-0" />
								<span class="text-secondary text-sm">{blocker.label}</span>
							</div>
							{#if isExpanded}
								<ChevronDown class="text-tertiary h-4 w-4 flex-shrink-0" />
							{:else}
								<ChevronRight class="text-tertiary h-4 w-4 flex-shrink-0" />
							{/if}
						</button>

						{#if isExpanded}
							<div class="px-3 py-2">
								{#if blockerId === 'compatibility'}
									<CompatibilityChecker {useCase} />
								{:else if blockerId === 'something_else'}
									<FeedbackForm blocker="something_else" />
								{:else}
									<div class="space-y-2">
										<InlineInfo title="" body={blocker.description} />
										{#if blocker.linkUrl}
											<!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
											<a
												href={blocker.linkUrl}
												target="_blank"
												rel="external noopener noreferrer"
												class="text-primary flex items-center gap-1 text-sm hover:underline"
											>
												{blocker.linkText}
												<ExternalLink class="h-3 w-3" />
											</a>
										{/if}
									</div>
								{/if}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</div>

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex justify-end">
				<button type="button" class="btn-primary" on:click={handleContinueSetup}>
					{onboarding_continueSetup()}
				</button>
			</div>
		</div>
	{/snippet}
</GenericModal>
