<script lang="ts">
	import { createForm, type AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import { type UseCase, type SetupRequest, getUseCases } from '../../types/base';
	import { required, max, min } from '#lib/shared/components/forms/validators.ts';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		auth_scanopyLogo,
		common_continue,
		common_settingUp,
		onboarding_licenseOrgHelp,
		onboarding_licenseOrgTitle,
		onboarding_visualizeCompany,
		onboarding_visualizeHomelab,
		onboarding_visualizeMsp
	} from '#lib/paraglide/messages.js';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onSubmit: (formData: SetupRequest) => void;
		useCase?: UseCase | null;
		/** A self-hosted license buyer: the org holds the license, so it skips the
		 *  site and is described as a license account, not a network to visualize. */
		licenseSignup?: boolean;
	}

	let {
		isOpen = false,
		onClose,
		onSubmit,
		useCase = null,
		licenseSignup = false
	}: Props = $props();

	let askSite = $derived(!licenseSignup);

	let loading = $state(false);

	// Get use case config (default to internal_it)
	let useCaseConfig = $derived(useCase ? getUseCases()[useCase] : getUseCases().internal_it);

	// Initialize from store (for back navigation persistence)
	const storeState = onboardingStore.getState();

	function getDefaultValues() {
		const storedSite = storeState.site;
		return {
			organizationName: storeState.organizationName || '',
			site: storedSite.name || ''
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			const formValues = value as Record<string, string | boolean>;
			const site = askSite ? { name: (formValues.site as string)?.trim() } : null;

			const formData: SetupRequest = {
				organization_name: (formValues.organizationName as string).trim(),
				site
			};

			trackEvent('onboarding_org_sites_selected', {
				sites_count: site ? 1 : 0,
				use_case: useCase
			});

			// Update store with final values
			onboardingStore.setOrganizationName(formData.organization_name);
			if (site) onboardingStore.setSite(site);

			onSubmit(formData);
		}
	}));

	async function handleSubmit() {
		await submitForm(form);
	}

	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
	}

	let title = $derived(
		licenseSignup
			? onboarding_licenseOrgTitle()
			: useCase === 'msp'
				? onboarding_visualizeMsp()
				: useCase === 'internal_it'
					? onboarding_visualizeCompany()
					: onboarding_visualizeHomelab()
	);

	let actions = $derived<ModalAction[]>([
		{
			role: 'primary',
			label: loading ? common_settingUp() : common_continue(),
			onClick: handleSubmit,
			disabled: loading
		}
	]);
</script>

<GenericModal
	{isOpen}
	{title}
	size="lg"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={false}
	showBackdrop={false}
	preventCloseOnClickOutside={true}
	centerTitle={true}
	{actions}
	footerLayout="stack"
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-6">
				<form.Field
					name="organizationName"
					validators={{
						onBlur: ({ value }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={useCaseConfig.orgLabel}
							id="organizationName"
							placeholder={useCaseConfig.orgPlaceholder}
							required={true}
							helpText={licenseSignup ? onboarding_licenseOrgHelp() : useCaseConfig.orgHelp}
							{field}
							autofocus
						/>
					{/snippet}
				</form.Field>

				{#if askSite}
					<div class="space-y-4">
						<div class="flex items-center gap-2">
							<div class="flex-1">
								<form.Field
									name="site"
									validators={{
										onBlur: ({ value }: { value: string }) => required(value) || min(1)(value)
									}}
								>
									{#snippet children(field: AnyFieldApi)}
										<TextInput
											label={useCaseConfig.siteLabel}
											id="site-0"
											{field}
											required={true}
											placeholder={useCaseConfig.sitePlaceholder}
											helpText={useCaseConfig.siteHelp}
										/>
									{/snippet}
								</form.Field>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>
	</form>
</GenericModal>
