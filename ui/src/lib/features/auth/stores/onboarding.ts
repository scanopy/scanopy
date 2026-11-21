import { writable, get } from 'svelte/store';
import type { UseCase, SiteSetup, SignupIntent } from '../types/base';
import type { PlanPickerHosting } from '#lib/features/billing/types.ts';

export interface OnboardingState {
	useCase: UseCase | null;
	/** The text typed for "Something else" as the use case. */
	useCaseOther: string | null;
	/** What the signup wants Scanopy to do. `null` until answered, empty when skipped. */
	intents: SignupIntent[] | null;
	/** The text typed for "Something else" as an intent. */
	intentOther: string | null;
	/** Where Scanopy runs, chosen at signup (preselected from `?hosting=`). Opens the plan picker on that tab. */
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

export const ONBOARDING_STEPS = ['hosting', 'use_case', 'intents', 'setup', 'register'] as const;
export type OnboardingStep = (typeof ONBOARDING_STEPS)[number];

/**
 * The new-org signup steps, in order. A server that sells both cloud and
 * self-hosted plans asks where Scanopy runs first. Every signup, a self-hosted
 * license buyer included, then says whose network it is and what it wants done.
 */
export function onboardingSteps(asksHosting: boolean): OnboardingStep[] {
	return ONBOARDING_STEPS.filter((step) => asksHosting || step !== 'hosting');
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
	useCaseOther: null,
	intents: null,
	intentOther: null,
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

		/** The use case, and its "Something else" text (`null` for any other use case). */
		setUseCase: (useCase: UseCase, useCaseOther: string | null = null) =>
			updateAndPersist((state) => ({
				...state,
				useCase,
				useCaseOther
			})),

		/** The intents, and the "Something else" text. An empty list records a skip. */
		setIntents: (intents: SignupIntent[], intentOther: string | null = null) =>
			update((state) => ({
				...state,
				intents,
				intentOther
			})),

		setHosting: (hosting: PlanPickerHosting) =>
			updateAndPersist((state) => ({
				...state,
				hosting
			})),

		// Once a plan is picked, the signup's hosting choice has done its job.
		// Clearing it stops it steering a later signup in this browser.
		clearHosting: () =>
			updateAndPersist((state) => ({
				...state,
				hosting: null
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
