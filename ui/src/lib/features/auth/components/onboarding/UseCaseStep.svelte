<script lang="ts">
	import { tick } from 'svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { AlertTriangle, Home, Building2, Users } from 'lucide-svelte';
	import { type UseCase, USE_CASES } from '../../types/base';
	import { useConfigQuery, isCloud, isCommunity } from '$lib/shared/stores/config-query';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import {
		auth_scanopyLogo,
		common_continue,
		common_other,
		common_reddit,
		common_youtube,
		onboarding_alreadyHaveAccount,
		onboarding_commercialNoticeBody,
		onboarding_commercialNoticeTitle,
		onboarding_haveQuestionsFirst,
		onboarding_howDidYouHear,
		onboarding_howWillYouUse,
		onboarding_logInHere,
		onboarding_readyToScan,
		onboarding_referralSource_blogArticle,
		onboarding_referralSource_hackerNews,
		onboarding_referralSource_otherPlaceholder,
		onboarding_referralSource_searchEngine,
		onboarding_referralSource_selfHosted,
		onboarding_referralSource_socialMedia,
		onboarding_referralSource_wordOfMouth,
		onboarding_tailorSetup,
		onboarding_understandContinue,
		onboarding_yesLetsGo
	} from '$lib/paraglide/messages';

	let {
		isOpen,
		onNext,
		onBlockerFlow,
		onClose,
		onSwitchToLogin = null
	}: {
		isOpen: boolean;
		onNext: () => void;
		onBlockerFlow: () => void;
		onClose: () => void;
		onSwitchToLogin?: (() => void) | null;
	} = $props();

	const configQuery = useConfigQuery();
	let configData = $derived(configQuery.data);

	let selectedUseCase = $state<UseCase | null>(null);
	let showLicenseWarning = $state(false);
	let scrollContainerEl = $state<HTMLElement | undefined>(undefined);

	// Role options
	const roleOptions = [
		{ value: '', label: 'Select your role', disabled: true },
		{ value: 'it_admin', label: 'IT Admin' },
		{ value: 'network_engineer', label: 'Network Engineer' },
		{ value: 'devops', label: 'DevOps' },
		{ value: 'manager', label: 'Manager / Director' },
		{ value: 'executive', label: 'Owner / Executive' },
		{ value: 'other', label: 'Other' }
	];

	// Company size options
	const companySizeOptions = [
		{ value: '', label: 'Select company size', disabled: true },
		{ value: '1-10', label: '1-10 employees' },
		{ value: '11-25', label: '11-25 employees' },
		{ value: '26-50', label: '26-50 employees' },
		{ value: '51-100', label: '51-100 employees' },
		{ value: '101-250', label: '101-250 employees' },
		{ value: '251-500', label: '251-500 employees' },
		{ value: '501-1000', label: '501-1000 employees' },
		{ value: '1001+', label: '1001+ employees' }
	];

	// Referral source options
	const referralSourceOptions = [
		{ value: '', label: onboarding_howDidYouHear(), disabled: true },
		{ value: 'search_engine', label: onboarding_referralSource_searchEngine() },
		{ value: 'youtube', label: common_youtube() },
		{ value: 'blog_article', label: onboarding_referralSource_blogArticle() },
		{ value: 'reddit', label: common_reddit() },
		{ value: 'hacker_news', label: onboarding_referralSource_hackerNews() },
		{ value: 'social_media', label: onboarding_referralSource_socialMedia() },
		{ value: 'word_of_mouth', label: onboarding_referralSource_wordOfMouth() },
		{ value: 'self_hosted', label: onboarding_referralSource_selfHosted() },
		{ value: 'other', label: common_other() }
	];

	// Icons for each use case (kept separate from types for flexibility)
	const useCaseIcons: Record<UseCase, typeof Home> = {
		homelab: Home,
		company: Building2,
		msp: Users
	};

	// Use case IDs for iteration
	const useCaseIds: UseCase[] = ['homelab', 'company', 'msp'];

	// Show business fields for company/msp
	let showBusinessFields = $derived(selectedUseCase === 'company' || selectedUseCase === 'msp');

	// Show Cloud-only qualification fields
	let showCloudFields = $derived(configData && isCloud(configData));

	// Form for business qualification fields
	const form = createForm(() => ({
		defaultValues: {
			role: '',
			companySize: '',
			referralSource: '',
			referralSourceOther: ''
		}
	}));

	// Track referral source value reactively for the "other" text field
	// (form.state.values is NOT tracked by $derived)
	let referralSourceValue = $state('');
	$effect(() => {
		return form.store.subscribe(() => {
			referralSourceValue = form.state.values.referralSource;
		});
	});

	function selectUseCase(useCase: UseCase) {
		selectedUseCase = useCase;

		// Reset business fields when switching to homelab
		if (useCase === 'homelab') {
			form.reset({ role: '', companySize: '', referralSource: '', referralSourceOther: '' });
		}

		// For Community self-hosted + Company/MSP: show license warning
		if (configData && isCommunity(configData) && (useCase === 'company' || useCase === 'msp')) {
			showLicenseWarning = true;
		} else {
			showLicenseWarning = false;
		}

		// Auto-scroll to bottom of modal after DOM updates
		tick().then(() => {
			if (scrollContainerEl) {
				scrollContainerEl.scrollTo({ top: scrollContainerEl.scrollHeight, behavior: 'smooth' });
			}
		});
	}

	function handleLicenseAcknowledge() {
		showLicenseWarning = false;
	}

	function saveBusinessFields() {
		const values = form.state.values;
		if (showBusinessFields) {
			onboardingStore.setJobTitle(values.role || null);
			onboardingStore.setCompanySize(values.companySize || null);
		}
		if (showCloudFields) {
			onboardingStore.setReferralSource(
				values.referralSource || null,
				values.referralSourceOther || null
			);
		}
	}

	// Self hosted handlers
	function handleContinue() {
		if (!selectedUseCase) return;
		saveBusinessFields();
		const values = form.state.values;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase,
			role: values.role || undefined,
			company_size: values.companySize || undefined,
			referral_source: values.referralSource || undefined,
			referral_source_other: values.referralSourceOther || undefined
		});
		onboardingStore.setUseCase(selectedUseCase);
		onNext();
	}

	// Cloud handlers
	function handleReadyYes() {
		if (!selectedUseCase) return;
		saveBusinessFields();
		const values = form.state.values;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase,
			role: values.role || undefined,
			company_size: values.companySize || undefined,
			referral_source: values.referralSource || undefined,
			referral_source_other: values.referralSourceOther || undefined
		});
		trackEvent('onboarding_ready_to_scan', { ready: true, use_case: selectedUseCase });
		onboardingStore.setUseCase(selectedUseCase);
		onboardingStore.setReadyToScan(true);
		onNext();
	}

	function handleReadyNo() {
		if (!selectedUseCase) return;
		saveBusinessFields();
		const values = form.state.values;
		trackEvent('onboarding_use_case_selected', {
			use_case: selectedUseCase,
			role: values.role || undefined,
			company_size: values.companySize || undefined,
			referral_source: values.referralSource || undefined,
			referral_source_other: values.referralSourceOther || undefined
		});
		trackEvent('onboarding_ready_to_scan', { ready: false, use_case: selectedUseCase });
		onboardingStore.setUseCase(selectedUseCase);
		onboardingStore.setReadyToScan(false);
		onBlockerFlow();
	}

	let isCloudDeployment = $derived(configData && isCloud(configData));
	let canProceed = $derived(selectedUseCase !== null && !showLicenseWarning);
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
		<div class="flex-1 overflow-y-auto p-6" bind:this={scrollContainerEl}>
			<div class="space-y-6">
				<p class="text-secondary text-center text-sm">{onboarding_tailorSetup()}</p>

				<!-- Use Case Cards -->
				<div class="grid gap-3">
					{#each useCaseIds as useCaseId (useCaseId)}
						{@const useCaseConfig = USE_CASES[useCaseId]}
						{@const isSelected = selectedUseCase === useCaseId}
						{@const Icon = useCaseIcons[useCaseId]}
						<button
							type="button"
							class="card flex items-center gap-4 p-4 text-left transition-all {isSelected
								? `ring-2 ${useCaseConfig.colors.ring}`
								: 'hover:bg-gray-800'}"
							onclick={() => selectUseCase(useCaseId)}
						>
							<div
								class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {isSelected
									? `${useCaseConfig.colors.bg} ${useCaseConfig.colors.text}`
									: 'bg-gray-700 text-gray-400'}"
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

				<!-- Cloud-only qualification fields -->
				{#if showCloudFields}
					<!-- Business Qualification Fields (Company/MSP only) -->
					{#if showBusinessFields}
						<div class="card card-static">
							<p class="text-secondary text-sm">Help us tailor your experience:</p>

							<div class="grid gap-4 sm:grid-cols-2">
								<form.Field name="role">
									{#snippet children(field)}
										<SelectInput label="Your role" id="role" {field} options={roleOptions} />
									{/snippet}
								</form.Field>

								<form.Field name="companySize">
									{#snippet children(field)}
										<SelectInput
											label="Company size"
											id="company-size"
											{field}
											options={companySizeOptions}
										/>
									{/snippet}
								</form.Field>
							</div>
						</div>
					{/if}

					<!-- Referral Source (all use cases on Cloud) -->
					{#if selectedUseCase}
						<div class="card card-static">
							<form.Field name="referralSource">
								{#snippet children(field)}
									<SelectInput
										label={onboarding_howDidYouHear()}
										id="referral-source"
										{field}
										options={referralSourceOptions}
									/>
									{#if referralSourceValue === 'other'}
										<div class="mt-3">
											<form.Field name="referralSourceOther">
												{#snippet children(otherField)}
													<TextInput
														label=""
														id="referral-source-other"
														field={otherField}
														placeholder={onboarding_referralSource_otherPlaceholder()}
													/>
												{/snippet}
											</form.Field>
										</div>
									{/if}
								{/snippet}
							</form.Field>
						</div>
					{/if}
				{/if}

				<!-- License Warning (Community + Company/MSP) -->
				{#if showLicenseWarning}
					<div class="rounded-lg border border-yellow-600/30 bg-yellow-900/20 p-4">
						<div class="flex items-start gap-2">
							<AlertTriangle class="mt-0.5 h-4 w-4 shrink-0 text-warning" />
							<div class="flex-1">
								<p class="text-sm font-medium text-warning">{onboarding_commercialNoticeTitle()}</p>
								<!-- svelte-ignore a11y -- trusted: i18n content with HTML links -->
								<p class="mt-1 text-sm text-warning">
									<!-- eslint-disable-next-line svelte/no-at-html-tags -- trusted: i18n content with HTML links -->
									{@html onboarding_commercialNoticeBody()}
								</p>
								<button type="button" class="btn-primary mt-4" onclick={handleLicenseAcknowledge}>
									{onboarding_understandContinue()}
								</button>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				{#if isCloudDeployment}
					<!-- Cloud: Show ready to scan buttons (disabled until use case selected) -->
					<p class="text-secondary text-center text-sm">
						{onboarding_readyToScan()}
					</p>
					<div class="flex gap-3">
						<button
							type="button"
							class="btn-secondary flex-1"
							disabled={!canProceed}
							onclick={handleReadyNo}
						>
							{onboarding_haveQuestionsFirst()}
						</button>
						<button
							type="button"
							class="btn-primary flex-1"
							disabled={!canProceed}
							onclick={handleReadyYes}
						>
							{onboarding_yesLetsGo()}
						</button>
					</div>
				{:else}
					<!-- Self-hosted: Show continue button -->
					<div class="flex justify-end">
						<button
							type="button"
							class="btn-primary"
							disabled={!canProceed}
							onclick={handleContinue}
						>
							{common_continue()}
						</button>
					</div>
				{/if}

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
