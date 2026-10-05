import { writable, get } from 'svelte/store';
import type { UseCase, SiteSetup } from '../types/base';
import type { PlanPickerHosting } from '$lib/features/billing/types';

export interface OnboardingState {
	useCase: UseCase | null;
	/** Plan-picker tab requested at signup (`?hosting=self_hosted`). */
	hosting: PlanPickerHosting | null;
	organizationName: string;
	site: SiteSetup;
	populateSeedData: boolean;
}

const STORAGE_KEY = 'scanopy_onboarding';

/**
 * Whether signup asks for a first site. A self-hosted license buyer on
 * cloud skips it: their org gets a site only if it later moves to a cloud
 * plan. A self-hosted instance always asks, because nothing creates one later.
 */
export function asksForSite(hosting: PlanPickerHosting | null, cloudDeployment: boolean) {
	return !(cloudDeployment && hosting === 'self_hosted');
}

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
	site: { name: '' },
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
				site: { name: '' },
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

		setSite: (site: SiteSetup) =>
			update((state) => ({
				...state,
				site
			})),

		setSiteId: (siteId: string) =>
			update((state) => ({
				...state,
				site: { ...state.site, id: siteId }
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
