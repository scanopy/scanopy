<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { type UseCase, getUseCases } from '../../types/base';
	import { useConfigQuery, isCommunity, isCloud } from '#lib/shared/stores/config-query.ts';
	import { asksForSite, onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import { useCases } from '#lib/shared/stores/metadata.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import MessageWithLinks from '#lib/shared/components/data/MessageWithLinks.svelte';
	import { slot } from '#lib/shared/components/data/MessageWithSlots.svelte';
	import { PRICING_URL } from '#lib/shared/utils/site-links.ts';
	import { otherAnswer } from './otherAnswer';
	import billingPlansJson from '#lib/data/billing-plans.json';
	import {
		auth_scanopyLogo,
		common_back,
		common_continue,
		onboarding_alreadyHaveAccount,
		onboarding_commercialNoticeBody,
		onboarding_commercialOfferingLinkText,
		onboarding_commercialNoticeTitle,
		onboarding_continueLicenseTrial,
		onboarding_homelabLicenseNoticeBody,
		onboarding_homelabLicenseNoticeTitle,
		onboarding_howWillYouUse,
		onboarding_logInHere,
		onboarding_somethingElseLabel,
		onboarding_somethingElsePlaceholder,
		onboarding_understandContinue,
		onboarding_useCommunity
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

	/** Where a home user installs Scanopy Community. */
	const COMMUNITY_INSTALL_URL =
		'https://scanopy.net/docs/self-hosted-server/installation/?edition=community';

	/** Community's seat and site limits, read from the plan fixture so the notice follows them. */
	const community = billingPlansJson.find((plan) => plan.id === 'Community')?.metadata;

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	const useCaseIds = useCases.getItems().map((item) => item.id as UseCase);

	let selectedUseCase = $state<UseCase | null>($onboardingStore.useCase);

	/** A notice that stands between the choice and the next step, in place of the options. */
	type Notice = 'commercial' | 'homelab_license';
	let notice = $state<Notice | null>(null);
	/** The choice the notice is about, sent once the user goes on. */
	let pending = $state<{ useCase: UseCase; other: string | null } | null>(null);

	const form = createForm(() => ({
		defaultValues: { use_case_other: $onboardingStore.useCaseOther ?? '' },
		onSubmit: ({ value }) => choose('other', value.use_case_other.trim() || null)
	}));

	/** The notice a choice needs before the next step, if any. */
	function noticeFor(useCase: UseCase): Notice | null {
		if (!configData) return null;
		// Scanopy Community is licensed for home use; anyone else is pointed at the license.
		if (isCommunity(configData) && useCase !== 'homelab') return 'commercial';
		// A license buyer on cloud documenting their own network: Community is free for that.
		const licenseSignup = !asksForSite($onboardingStore.hosting, isCloud(configData));
		if (licenseSignup && useCase === 'homelab') return 'homelab_license';
		return null;
	}

	function choose(useCase: UseCase, other: string | null = null) {
		const needed = noticeFor(useCase);
		if (needed) {
			pending = { useCase, other };
			notice = needed;
			return;
		}
		proceed(useCase, other);
	}

	function proceed(useCase: UseCase, other: string | null) {
		trackEvent('onboarding_use_case_selected', {
			use_case: useCase,
			// Plan-picker tab the signup link asked for (`?hosting=`), or null.
			hosting: onboardingStore.getState().hosting
		});
		onboardingStore.setUseCase(useCase, other);
		onNext();
	}

	function selectUseCase(useCase: UseCase) {
		selectedUseCase = useCase;
		// "Something else" waits for its text; every other choice goes on at once.
		if (useCase !== 'other') choose(useCase);
	}

	function backToOptions() {
		notice = null;
		pending = null;
	}

	function goOn() {
		if (!pending) return;
		if (notice === 'homelab_license') {
			trackEvent('onboarding_homelab_license_notice', { action: 'continued_trial' });
		}
		proceed(pending.useCase, pending.other);
	}

	function goToCommunity() {
		trackEvent('onboarding_homelab_license_notice', { action: 'went_to_community' });
		window.open(COMMUNITY_INSTALL_URL, '_blank', 'noopener');
	}

	let actions = $derived.by<ModalAction[]>(() => {
		if (notice === 'commercial') {
			return [
				{ role: 'cancel', label: common_back(), onClick: backToOptions },
				{ role: 'primary', label: onboarding_understandContinue(), onClick: goOn }
			];
		}
		if (notice === 'homelab_license') {
			return [
				{ role: 'cancel', label: common_back(), onClick: backToOptions },
				{ role: 'secondary', label: onboarding_continueLicenseTrial(), onClick: goOn },
				{ role: 'primary', label: onboarding_useCommunity(), onClick: goToCommunity }
			];
		}
		if (selectedUseCase === 'other') {
			return [{ role: 'primary', label: common_continue(), onClick: () => submitForm(form) }];
		}
		return [];
	});

	let pendingUseCase = $derived(pending?.useCase ?? null);
</script>

<GenericModal
	{isOpen}
	title={onboarding_howWillYouUse()}
	{onClose}
	size="xl"
	centerTitle={true}
	showBackdrop={false}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
	{actions}
	footerLayout="stack"
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
			{#if notice && pendingUseCase}
				<!-- The choice, then what it means. The buttons sit in the footer. -->
				{@const colors = getUseCases()[pendingUseCase].colors}
				{@const Icon = useCases.getIconComponent(pendingUseCase)}
				<div class="space-y-4">
					<div class="card-static card flex items-center gap-4 p-4 ring-2 {colors.ring}">
						<div
							class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {colors.bg} {colors.text}"
						>
							<Icon class="h-5 w-5" />
						</div>
						<div class="font-medium text-primary">{useCases.getName(pendingUseCase)}</div>
					</div>
					{#if notice === 'commercial'}
						<InlineWarning title={onboarding_commercialNoticeTitle()}>
							<p class="mt-1 text-sm">
								<MessageWithLinks
									text={onboarding_commercialNoticeBody({ link: slot('link') })}
									links={{
										link: { href: PRICING_URL, label: onboarding_commercialOfferingLinkText() }
									}}
								/>
							</p>
						</InlineWarning>
					{:else}
						<InlineInfo
							title={onboarding_homelabLicenseNoticeTitle()}
							body={onboarding_homelabLicenseNoticeBody({
								seats: community?.included_seats ?? 1,
								sites: community?.included_sites ?? 1
							})}
						/>
					{/if}
				</div>
			{:else}
				<div class="grid gap-3">
					{#each useCaseIds as useCaseId (useCaseId)}
						{@const colors = getUseCases()[useCaseId].colors}
						{@const isSelected = selectedUseCase === useCaseId}
						{@const Icon = useCases.getIconComponent(useCaseId)}
						{@const description = useCases.getDescription(useCaseId)}
						<div class="card p-0 transition-all {isSelected ? `ring-2 ${colors.ring}` : ''}">
							<button
								type="button"
								class="flex w-full items-center gap-4 rounded-lg p-4 text-left {isSelected
									? ''
									: 'hover:bg-gray-100 dark:hover:bg-gray-800'}"
								onclick={() => selectUseCase(useCaseId)}
							>
								<div
									class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {isSelected
										? `${colors.bg} ${colors.text}`
										: 'bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400'}"
								>
									<Icon class="h-5 w-5" />
								</div>
								<div>
									<div class="font-medium text-primary">{useCases.getName(useCaseId)}</div>
									{#if description}
										<div class="text-sm text-secondary">{description}</div>
									{/if}
								</div>
							</button>
							{#if isSelected && useCaseId === 'other'}
								<!-- The answer opens inside the card the user clicked. -->
								<form
									class="px-4 pb-4"
									onsubmit={(e) => {
										e.preventDefault();
										e.stopPropagation();
										submitForm(form);
									}}
								>
									<form.Field name="use_case_other" validators={{ onSubmit: otherAnswer }}>
										{#snippet children(field)}
											<TextInput
												label={onboarding_somethingElseLabel()}
												id="use-case-other"
												{field}
												placeholder={onboarding_somethingElsePlaceholder()}
												autofocus
											/>
										{/snippet}
									</form.Field>
								</form>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</GenericModal>
