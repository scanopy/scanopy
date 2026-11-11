<script lang="ts">
	import { Cloud, Server } from '@lucide/svelte';
	import { onboardingStore } from '../../stores/onboarding';
	import type { PlanPickerHosting } from '#lib/features/billing/types.ts';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import {
		auth_scanopyLogo,
		onboarding_alreadyHaveAccount,
		onboarding_hostingCloudDescription,
		onboarding_hostingCloudLabel,
		onboarding_hostingSelfHostedDescription,
		onboarding_hostingSelfHostedLabel,
		onboarding_logInHere,
		onboarding_whereWillScanopyRun
	} from '#lib/paraglide/messages.js';

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

	// The signup link's `?hosting=` (or an earlier pick), shown as selected so a
	// website license buyer sees their choice and confirms it.
	const preselected = onboardingStore.getState().hosting;

	const options: {
		id: PlanPickerHosting;
		icon: typeof Cloud;
		label: () => string;
		description: () => string;
	}[] = [
		{
			id: 'cloud',
			icon: Cloud,
			label: () => onboarding_hostingCloudLabel(),
			description: () => onboarding_hostingCloudDescription()
		},
		{
			id: 'self_hosted',
			icon: Server,
			label: () => onboarding_hostingSelfHostedLabel(),
			description: () => onboarding_hostingSelfHostedDescription()
		}
	];

	function select(hosting: PlanPickerHosting) {
		trackEvent('onboarding_hosting_selected', { hosting, preselected });
		onboardingStore.setHosting(hosting);
		onNext();
	}
</script>

<GenericModal
	{isOpen}
	title={onboarding_whereWillScanopyRun()}
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

	{#snippet footerNote()}
		{#if onSwitchToLogin}
			<p class="text-center text-sm text-secondary">
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
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-y-auto p-6">
			<div class="space-y-6">
				<div class="grid gap-3">
					{#each options as option (option.id)}
						{@const isSelected = preselected === option.id}
						{@const Icon = option.icon}
						<button
							type="button"
							class="card flex items-center gap-4 p-4 text-left transition-all {isSelected
								? 'ring-2 ring-blue-500'
								: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
							onclick={() => select(option.id)}
						>
							<div
								class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {isSelected
									? 'bg-blue-500/20 text-blue-400'
									: 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'}"
							>
								<Icon class="h-5 w-5" />
							</div>
							<div>
								<div class="font-medium text-primary">{option.label()}</div>
								<div class="text-sm text-secondary">{option.description()}</div>
							</div>
						</button>
					{/each}
				</div>
			</div>
		</div>
	</div>
</GenericModal>
