import { writable, get } from 'svelte/store';
import type { UseCase, NetworkSetup } from '../types/base';
import type { PlanPickerHosting } from '$lib/features/billing/types';

export interface OnboardingState {
	useCase: UseCase | null;
	/** Plan-picker tab requested at signup (`?hosting=self_hosted`). */
	hosting: PlanPickerHosting | null;
	organizationName: string;
	network: NetworkSetup;
	populateSeedData: boolean;
}

const STORAGE_KEY = 'scanopy_onboarding';

// Fields to persist to localStorage (for billing page autofill)
interface PersistedState {
	useCase: UseCase | null;
	hosting: PlanPickerHosting | null;
}

function loadPersistedState(): PersistedState {
	if (typeof window === 'undefined') {
		return { useCase: null, hosting: null };
	}
	try {
		const stored = localStorage.getItem(STORAGE_KEY);
		if (stored) {
			const parsed = JSON.parse(stored);
			return {
				useCase: parsed.useCase ?? null,
				hosting: parsed.hosting ?? null
			};
		}
	} catch {
		// Ignore localStorage errors
	}
	return { useCase: null, hosting: null };
}

function savePersistedState(state: PersistedState): void {
	if (typeof window === 'undefined') return;
	try {
		localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
	} catch {
		// Ignore localStorage errors
	}
}

const persisted = loadPersistedState();

const initialState: OnboardingState = {
	useCase: persisted.useCase,
	hosting: persisted.hosting,
	organizationName: '',
	network: { name: '' },
	populateSeedData: true
};

function createOnboardingStore() {
	const { subscribe, update } = writable<OnboardingState>({ ...initialState });

	// Helper to update and persist
	function updateAndPersist(updater: (state: OnboardingState) => OnboardingState): void {
		update((state) => {
			const newState = updater(state);
			savePersistedState({
				useCase: newState.useCase,
				hosting: newState.hosting
			});
			return newState;
		});
	}

	return {
		subscribe,
		// Reset clears most state but preserves useCase and hosting for billing page
		reset: () =>
			updateAndPersist((state) => ({
				...initialState,
				network: { name: '' },
				useCase: state.useCase, // Preserve for billing page
				hosting: state.hosting // Preserve for billing page
			})),

		setUseCase: (useCase: UseCase) =>
			updateAndPersist((state) => ({
				...state,
				useCase
			})),

		setHosting: (hosting: PlanPickerHosting) =>
			updateAndPersist((state) => ({
				...state,
				hosting
			})),

		setOrganizationName: (name: string) =>
			update((state) => ({
				...state,
				organizationName: name
			})),

		setNetwork: (network: NetworkSetup) =>
			update((state) => ({
				...state,
				network
			})),

		setNetworkId: (networkId: string) =>
			update((state) => ({
				...state,
				network: { ...state.network, id: networkId }
			})),

		setPopulateSeedData: (populate: boolean) =>
			update((state) => ({
				...state,
				populateSeedData: populate
			})),

		// Get the current state synchronously
		getState: () => get({ subscribe })
	};
}

export const onboardingStore = createOnboardingStore();
