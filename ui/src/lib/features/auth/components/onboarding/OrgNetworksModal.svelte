<script lang="ts">
	import { createForm, type AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { type UseCase, type SetupRequest, getUseCases } from '../../types/base';
	import { required, max, min } from '$lib/shared/components/forms/validators';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		auth_scanopyLogo,
		common_continue,
		common_settingUp,
		onboarding_visualizeCompany,
		onboarding_visualizeHomelab,
		onboarding_visualizeMsp
	} from '$lib/paraglide/messages';

	interface Props {
		isOpen?: boolean;
		onClose: () => void;
		onSubmit: (formData: SetupRequest) => void;
		useCase?: UseCase | null;
		/** False for a self-hosted license buyer, who skips the network step. */
		askNetwork?: boolean;
	}

	let { isOpen = false, onClose, onSubmit, useCase = null, askNetwork = true }: Props = $props();

	let loading = $state(false);

	// Get use case config (default to internal_it)
	let useCaseConfig = $derived(useCase ? getUseCases()[useCase] : getUseCases().internal_it);

	// Initialize from store (for back navigation persistence)
	const storeState = onboardingStore.getState();

	function getDefaultValues() {
		const storedNetwork = storeState.network;
		return {
			organizationName: storeState.organizationName || '',
			network: storedNetwork.name || ''
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			const formValues = value as Record<string, string | boolean>;
			const network = askNetwork ? { name: (formValues.network as string)?.trim() } : null;

			const formData: SetupRequest = {
				organization_name: (formValues.organizationName as string).trim(),
				network
			};

			trackEvent('onboarding_org_networks_selected', {
				networks_count: network ? 1 : 0,
				use_case: useCase
			});

			// Update store with final values
			onboardingStore.setOrganizationName(formData.organization_name);
			if (network) onboardingStore.setNetwork(network);

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
		useCase === 'msp'
			? onboarding_visualizeMsp()
			: useCase === 'internal_it'
				? onboarding_visualizeCompany()
				: onboarding_visualizeHomelab()
	);
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
							helpText={useCaseConfig.orgHelp}
							{field}
						/>
					{/snippet}
				</form.Field>

				{#if askNetwork}
					<div class="space-y-4">
						<div class="flex items-center gap-2">
							<div class="flex-1">
								<form.Field
									name="network"
									validators={{
										onBlur: ({ value }: { value: string }) => required(value) || min(1)(value)
									}}
								>
									{#snippet children(field: AnyFieldApi)}
										<TextInput
											label={useCaseConfig.networkLabel}
											id="network-0"
											{field}
											required={true}
											placeholder={useCaseConfig.networkPlaceholder}
											helpText={useCaseConfig.networkHelp}
										/>
									{/snippet}
								</form.Field>
							</div>
						</div>
					</div>
				{/if}
			</div>
		</div>

		<div class="modal-footer">
			<div class="flex w-full flex-col gap-4">
				<button type="submit" disabled={loading} class="btn-primary w-full">
					{loading ? common_settingUp() : common_continue()}
				</button>
			</div>
		</div>
	</form>
</GenericModal>
