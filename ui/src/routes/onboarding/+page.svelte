<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { themeStore } from '$lib/shared/stores/theme.svelte';
	import { ChevronLeft } from 'lucide-svelte';
	import Toast from '$lib/shared/components/feedback/Toast.svelte';
	import OrgNetworksModal from '$lib/features/auth/components/onboarding/OrgNetworksModal.svelte';
	import RegisterModal from '$lib/features/auth/components/RegisterModal.svelte';
	import UseCaseStep from '$lib/features/auth/components/onboarding/UseCaseStep.svelte';
	import type { RegisterRequest, SetupRequest, UseCase } from '$lib/features/auth/types/base';
	import {
		useSetupMutation,
		useRegisterMutation,
		useOnboardingStepMutation,
		useOnboardingStateQuery
	} from '$lib/features/auth/queries';
	import { fetchOrganization } from '$lib/features/organizations/queries';
	import { navigate } from '$lib/shared/utils/navigation';
	import { resolve } from '$app/paths';
	import { onboardingStore } from '$lib/features/auth/stores/onboarding';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { pushError } from '$lib/shared/stores/feedback';
	import { auth_emailAlreadyInUse } from '$lib/paraglide/messages';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';

	// Show OIDC error from redirect if present
	onMount(() => {
		const error = $page.url.searchParams.get('error');
		const errorCode = $page.url.searchParams.get('error_code');
		if (error) {
			// If the error indicates a duplicate email, redirect to login
			if (errorCode === 'user_email_in_use') {
				pushError(auth_emailAlreadyInUse());
				setTimeout(() => goto(resolve('/login')), 1500);
			} else {
				pushError(error);
			}
			const url = new URL(window.location.href);
			url.searchParams.delete('error');
			url.searchParams.delete('error_code');
			window.history.replaceState({}, '', url.toString());
		}
	});

	// TanStack Query mutations and queries
	const setupMutation = useSetupMutation();
	const registerMutation = useRegisterMutation();
	const onboardingStepMutation = useOnboardingStepMutation();
	const onboardingStateQuery = useOnboardingStateQuery();

	const onboardingConfigQuery = useConfigQuery();
	let onboardingConfigData = $derived(onboardingConfigQuery.data);

	// URL params for invite flow
	let orgName = $derived($page.url.searchParams.get('org_name'));
	let invitedBy = $derived($page.url.searchParams.get('invited_by'));

	// Determine if this is an invite flow (skip to register)
	let isInviteFlow = $derived(!!invitedBy);

	// Step tracking
	type Step = 'use_case' | 'setup' | 'register';

	// Get initial step from URL params or default
	function getInitialStep(): Step {
		if ($page.url.searchParams.get('invited_by')) return 'register';
		return 'use_case';
	}

	let currentStep = $state<Step>(getInitialStep());
	let stepInitialized = $state(false);
	let lastPersistedStep = $state<Step | null>(null);

	// Restore step and store data from session on mount
	$effect(() => {
		if (!stepInitialized && onboardingStateQuery.data && !isInviteFlow) {
			const stateData = onboardingStateQuery.data;

			// Restore step
			if (stateData.step && isValidStep(stateData.step)) {
				currentStep = stateData.step as Step;
				lastPersistedStep = stateData.step as Step; // Don't re-persist this
			}

			// Restore use case
			if (stateData.use_case && isValidUseCase(stateData.use_case)) {
				onboardingStore.setUseCase(stateData.use_case as UseCase);
			}

			// Restore org name
			if (stateData.org_name) {
				onboardingStore.setOrganizationName(stateData.org_name);
			}

			// Restore network (with ID and name)
			if (stateData.network) {
				onboardingStore.setNetwork({
					id: stateData.network.id ?? undefined,
					name: stateData.network.name,
					snmp_enabled: stateData.network.snmp_enabled ?? false,
					snmp_version: stateData.network.snmp_version ?? undefined,
					snmp_community: stateData.network.snmp_community ?? undefined
				});
			}

			stepInitialized = true;
		}
	});

	// Helper to validate use case
	function isValidUseCase(useCase: string): useCase is UseCase {
		return ['homelab', 'company', 'msp'].includes(useCase);
	}

	// Helper to validate step
	function isValidStep(step: string): step is Step {
		return ['use_case', 'setup', 'register'].includes(step);
	}

	// Persist step to session whenever it changes
	$effect(() => {
		if (stepInitialized && !isInviteFlow && currentStep !== lastPersistedStep) {
			lastPersistedStep = currentStep;
			onboardingStepMutation.mutate({
				step: currentStep,
				use_case: useCase ?? undefined
			});
		}
	});

	// Get use case from store
	let useCase = $derived($onboardingStore.useCase);

	// Calculate total steps based on flow
	// use_case -> setup -> register = 3 steps
	// Invite: just register = 1 step
	let totalSteps = $derived(() => {
		if (isInviteFlow) return 1;
		return 3;
	});

	let currentStepNumber = $derived(() => {
		if (isInviteFlow) return 1;

		const stepMap: Record<Step, number> = {
			use_case: 1,
			setup: 2,
			register: 3
		};
		return stepMap[currentStep];
	});

	// Note: Auth check is handled by +layout.svelte

	function handleUseCaseNext() {
		currentStep = 'setup';
	}

	async function handleSetupSubmit(formData: SetupRequest) {
		try {
			// Submit setup data to backend (stored in session)
			const result = await setupMutation.mutateAsync(formData);
			// Update store with network ID
			onboardingStore.setNetworkId(result.network_id);

			// Track onboarding modal completion
			trackEvent('onboarding_modal_completed', {
				network_count: 1
			});

			currentStep = 'register';
		} catch {
			// Error handled by mutation
		}
	}

	function handleBack() {
		switch (currentStep) {
			case 'setup':
				currentStep = 'use_case';
				break;
			case 'register':
				currentStep = 'setup';
				break;
		}
	}

	async function handleRegister(data: RegisterRequest, subscribed: boolean) {
		try {
			// Include marketing_opt_in in the registration request
			await registerMutation.mutateAsync({
				...data,
				marketing_opt_in: subscribed
			});

			// Before clearing onboarding store, get state for tracking
			const state = onboardingStore.getState();

			// Track successful registration with context
			trackEvent('onboarding_registration_completed', {
				use_case: state.useCase
			});

			// Fetch organization data before navigating
			await fetchOrganization();

			// Clear onboarding store
			onboardingStore.reset();

			// Navigate to main app — daemon prompt auto-opens via +page.svelte
			// for new orgs without daemons (both cloud and non-cloud)
			await navigate();
		} catch {
			// Error handled by mutation
		}
	}

	function handleSwitchToLogin() {
		goto(resolve('/login'));
	}

	function handleClose() {
		// Don't allow closing during onboarding
	}
</script>

<div class="relative flex min-h-screen flex-col items-center bg-[var(--color-bg-elevated)] p-4">
	<!-- Background image with overlay -->
	<div class="absolute inset-0 z-0">
		<div
			class="h-full w-full bg-cover bg-center bg-no-repeat blur-[2px]"
			style="background-image: url('/images/background-{themeStore.resolvedTheme}.webp')"
		></div>
		<div
			class="absolute inset-0 {themeStore.resolvedTheme === 'dark' ? 'bg-black/30' : 'bg-white/15'}"
		></div>
	</div>

	<!-- Progress Indicator - fixed position above modal (hidden for invite flow) -->
	{#if !isInviteFlow}
		<div class="fixed left-1/2 top-2 z-[200] -translate-x-1/2 sm:top-6">
			<div class="flex flex-col items-center gap-1">
				<div
					class="flex items-center gap-2 rounded-full bg-white/90 px-4 py-2 shadow-lg backdrop-blur-sm dark:bg-gray-800/90"
				>
					{#if currentStepNumber() > 1}
						<button
							type="button"
							onclick={handleBack}
							class="text-secondary hover:text-primary -ml-1 flex items-center transition-colors"
							aria-label="Go back"
						>
							<ChevronLeft class="h-4 w-4" />
						</button>
					{/if}
					<span class="text-secondary text-sm">
						Step {currentStepNumber()} of {totalSteps()}
					</span>
					<div class="flex gap-1">
						<!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
						{#each Array(totalSteps()) as _, i (i)}
							<div
								class="h-2 w-2 rounded-full transition-colors {i < currentStepNumber()
									? 'bg-primary-500'
									: 'bg-gray-300 dark:bg-gray-600'}"
							></div>
						{/each}
					</div>
				</div>
				{#if onboardingConfigData && isCloud(onboardingConfigData)}
					<span class="text-tertiary text-xs">No credit card required</span>
				{/if}
			</div>
		</div>
	{/if}

	<!-- Content container -->
	<div class="flex flex-1 items-center justify-center">
		<div class="relative z-10 w-full">
			{#if currentStep === 'use_case'}
				<!-- Use Case Selection Step -->
				<UseCaseStep
					isOpen={true}
					onNext={handleUseCaseNext}
					onClose={handleClose}
					onSwitchToLogin={handleSwitchToLogin}
				/>
			{:else if currentStep === 'setup'}
				<!-- Organization & Network Setup -->
				<OrgNetworksModal
					isOpen={true}
					onClose={handleClose}
					onSubmit={handleSetupSubmit}
					{useCase}
				/>
			{:else if currentStep === 'register'}
				<!-- Registration -->
				<RegisterModal
					isOpen={true}
					onRegister={handleRegister}
					onClose={handleClose}
					onSwitchToLogin={handleSwitchToLogin}
					{orgName}
					{invitedBy}
				/>
			{/if}
		</div>
	</div>

	<Toast />
</div>
