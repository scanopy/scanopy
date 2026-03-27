<script lang="ts">
	import { Home, Building2, Users, Boxes } from 'lucide-svelte';
	import { type UseCase, getUseCases } from '../../types/base';
	import { useConfigQuery, isCommunity } from '$lib/shared/stores/config-query';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import {
		auth_scanopyLogo,
		onboarding_alreadyHaveAccount,
		onboarding_commercialNoticeBody,
		onboarding_commercialNoticeTitle,
		onboarding_howWillYouUse,
		onboarding_logInHere,
		onboarding_tailorSetup,
		onboarding_understandContinue
	} from '$lib/paraglide/messages';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';

	let {
		isOpen,
		onNext,
		onClose,
		onSwitchToLogin = null
	}: {
		isOpen: boolean;
		onNext: () => void;
		onClose: () => void;
		onSwitchToLogin?: (() => void) | null;
	} = $props();

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let selectedUseCase = $state<UseCase | null>(null);
	let showLicenseWarning = $state(false);

	// Icons for each use case (kept separate from types for flexibility)
	const useCaseIcons: Record<UseCase, typeof Home> = {
		homelab: Home,
		company: Building2,
		msp: Users,
		other: Boxes
	};

	// Use case IDs for iteration
	const useCaseIds: UseCase[] = ['homelab', 'company', 'msp', 'other'];

	function submitAndProceed() {
		if (!selectedUseCase) return;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase
		});
		onboardingStore.setUseCase(selectedUseCase);
		onNext();
	}

	function selectUseCase(useCase: UseCase) {
		selectedUseCase = useCase;

		// For Community self-hosted + Company/MSP: show license warning
		if (configData && isCommunity(configData) && useCase !== 'homelab') {
			showLicenseWarning = true;
		} else {
			showLicenseWarning = false;
			submitAndProceed();
		}
	}

	function handleLicenseAcknowledge() {
		showLicenseWarning = false;
		submitAndProceed();
	}
</script>

<GenericModal
	{isOpen}
	title={onboarding_howWillYouUse()}
	{onClose}
	size="lg"
	centerTitle={true}
	showBackdrop={false}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-y-auto p-6">
			<div class="space-y-6">
				<p class="text-secondary text-center text-sm">{onboarding_tailorSetup()}</p>

				<!-- Use Case Cards -->
				<div class="grid gap-3">
					{#each useCaseIds as useCaseId (useCaseId)}
						{@const useCaseConfig = getUseCases()[useCaseId]}
						{@const isSelected = selectedUseCase === useCaseId}
						{@const Icon = useCaseIcons[useCaseId]}
						<button
							type="button"
							class="card flex items-center gap-4 p-4 text-left transition-all {isSelected
								? `ring-2 ${useCaseConfig.colors.ring}`
								: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
							onclick={() => selectUseCase(useCaseId)}
						>
							<div
								class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {isSelected
									? `${useCaseConfig.colors.bg} ${useCaseConfig.colors.text}`
									: 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'}"
							>
								<Icon class="h-5 w-5" />
							</div>
							<div>
								<div class="text-primary font-medium">{useCaseConfig.label}</div>
								<div class="text-secondary text-sm">{useCaseConfig.description}</div>
							</div>
						</button>
					{/each}
				</div>

				<!-- License Warning (Community + Company/MSP) -->
				{#if showLicenseWarning}
					<InlineWarning
						title={onboarding_commercialNoticeTitle()}
						body={onboarding_commercialNoticeBody()}
					/>
					<button type="button" class="btn-primary mt-4" onclick={handleLicenseAcknowledge}>
						{onboarding_understandContinue()}
					</button>
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if onSwitchToLogin}
					<p class="text-secondary text-center text-sm">
						{onboarding_alreadyHaveAccount()}
						<button
							type="button"
							onclick={onSwitchToLogin}
							class="font-medium text-blue-400 hover:text-blue-300"
						>
							{onboarding_logInHere()}
						</button>
					</p>
				{/if}
			</div>
		</div>
	</div>
</GenericModal>
