// `/pure`: importing this module injects nothing. Stripe.js loads only when
// `preloadStripe` first runs (see StripeCardForm for why that matters).
import { loadStripe } from '@stripe/stripe-js/pure';
import { untrack } from 'svelte';
import type {
	Stripe,
	StripeElements,
	StripePaymentElement,
	StripePaymentElementOptions
} from '@stripe/stripe-js';
import { buildStripeAppearance } from '#lib/shared/billing/stripe-appearance.ts';
import { useConfigQuery } from '#lib/shared/stores/config-query.ts';
import { useCurrentUserQuery } from '#lib/features/auth/queries.ts';
import { useCan } from '#lib/features/auth/access.ts';
import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
import { createSetupIntent } from '#lib/features/billing/queries.ts';

let stripePromise: Promise<Stripe | null> | null = null;
let loadedKey: string | null = null;

/**
 * Stripe.js for `publishableKey`, loaded once and shared. The payment dialog
 * awaits the same promise the surfaces that offer it started, so by the time
 * someone clicks Add Payment Method the script is already there. A failed
 * load is forgotten, so the next caller retries.
 */
export function preloadStripe(
	publishableKey: string | null | undefined
): Promise<Stripe | null> | null {
	if (!publishableKey) return null;
	if (stripePromise == null || loadedKey !== publishableKey) {
		loadedKey = publishableKey;
		stripePromise = loadStripe(publishableKey).catch((error: unknown) => {
			stripePromise = null;
			throw error;
		});
	}
	return stripePromise;
}

// One SetupIntent per org, made ahead of the payment dialog. An unconfirmed
// intent can be reused, so it is kept across opens until a card is saved.
let setupIntent: { organizationId: string; clientSecret: Promise<string> } | null = null;
// Bumped when the cached intent is spent, so surfaces still showing prefetch
// the next one rather than leaving the next open to fetch it.
let setupIntentGeneration = $state(0);
// The cached intent's client secret once it has arrived.
let readySetupIntent = $state<{ organizationId: string; clientSecret: string } | null>(null);
// How many surfaces that open the payment dialog are showing for an owner.
let warmSurfaces = $state(0);

/**
 * The client secret for `organizationId`'s payment form, created now unless
 * one is already in flight or ready. A failure is forgotten, so the next
 * caller retries.
 */
export function takeSetupIntent(organizationId: string): Promise<string> {
	if (setupIntent == null || setupIntent.organizationId !== organizationId) {
		const clientSecret = createSetupIntent().catch((error: unknown) => {
			if (setupIntent?.clientSecret === clientSecret) setupIntent = null;
			throw error;
		});
		setupIntent = { organizationId, clientSecret };
		void clientSecret.then(
			(secret) => {
				if (setupIntent?.clientSecret === clientSecret) {
					readySetupIntent = { organizationId, clientSecret: secret };
				}
			},
			() => {
				// The catch above has forgotten it.
			}
		);
	}
	return setupIntent.clientSecret;
}

/** Drop the cached SetupIntent, and the form built on it, once a card has been saved with it. */
export function clearSetupIntent(): void {
	setupIntent = null;
	readySetupIntent = null;
	preparedForm = null;
	setupIntentGeneration += 1;
}

/**
 * The payment dialog's head start: whether a surface that opens it is showing
 * for an owner (so the dialog keeps its card form mounted, drawn and hidden),
 * and the prefetched client secret that form mounts on, once it has arrived.
 */
export const paymentDialogPreload = {
	get warm(): boolean {
		return warmSurfaces > 0;
	},
	clientSecretFor(organizationId: string | undefined): string | null {
		return readySetupIntent != null && readySetupIntent.organizationId === organizationId
			? readySetupIntent.clientSecret
			: null;
	}
};

/** The Payment Element's options: the same in a prepared form and one built on open. */
export function paymentElementOptions(email: string | undefined): StripePaymentElementOptions {
	return {
		// `defaultCollapsed` left unset means Stripe expands whichever method it
		// thinks converts best, and then reports that method as selected before
		// the customer has touched anything. Open on the list instead, so
		// "selected" means they picked it.
		layout: { type: 'accordion', defaultCollapsed: true },
		defaultValues: email ? { billingDetails: { email } } : undefined
	};
}

export interface PreparedPaymentForm {
	clientSecret: string;
	elements: StripeElements;
	paymentElement: StripePaymentElement;
}

// The Elements group and Payment Element for the cached SetupIntent, built
// ahead of the dialog. Creating the group fetches Stripe's Elements session,
// the round trip the element otherwise waits on when the dialog opens.
let preparedForm: { clientSecret: string; form: Promise<PreparedPaymentForm | null> } | null = null;

/**
 * The payment form for `clientSecret`, built once and shared: the dialog
 * mounts the element prepared while the surface that opened it was showing.
 * Null when Stripe.js didn't load. A failure is forgotten, so the next caller
 * rebuilds.
 */
export function preparePaymentForm(
	publishableKey: string,
	clientSecret: string,
	email: string | undefined
): Promise<PreparedPaymentForm | null> {
	if (preparedForm == null || preparedForm.clientSecret !== clientSecret) {
		const form = (async () => {
			const stripe = await preloadStripe(publishableKey);
			if (!stripe) return null;
			// `loader: 'never'` disables Stripe's optimistic skeleton so the user
			// sees our native spinner until the element is fully ready (no flash
			// of placeholder cards). Appearance mirrors the app theme.
			const elements = stripe.elements({
				clientSecret,
				appearance: buildStripeAppearance(),
				loader: 'never'
			});
			const paymentElement = elements.create('payment', paymentElementOptions(email));
			return { clientSecret, elements, paymentElement };
		})().catch((error: unknown) => {
			if (preparedForm?.form === form) preparedForm = null;
			throw error;
		});
		preparedForm = { clientSecret, form };
	}
	return preparedForm.form;
}

/**
 * Get the payment dialog ready while a component that opens it is showing
 * it (`showing`, default always): Stripe.js once the config names the
 * publishable key, and, for an owner (the only role that can add a card),
 * the SetupIntent the form mounts on. Components that stay mounted while
 * hidden pass `showing`, so a page that offers no card fetches neither.
 */
export function usePreloadStripe(showing: () => boolean = () => true): void {
	const configQuery = useConfigQuery();
	const currentUserQuery = useCurrentUserQuery();
	const organizationQuery = useOrganizationQuery();
	const can = useCan();
	$effect(() => {
		if (!showing()) return;
		void setupIntentGeneration;
		const publishableKey = configQuery.data?.stripe_publishable_key;
		void preloadStripe(publishableKey)?.catch(() => {
			// StripeCardForm reports a failed load when it needs Stripe.
		});
		const organizationId = organizationQuery.data?.id;
		const email = currentUserQuery.data?.email;
		if (publishableKey && organizationId && can('Billing', 'Write')) {
			void takeSetupIntent(organizationId)
				.then((clientSecret) => preparePaymentForm(publishableKey, clientSecret, email))
				.catch(() => {
					// The API client toasts a failed intent, and the dialog retries
					// both when it opens.
				});
			untrack(() => (warmSurfaces += 1));
			return () => untrack(() => (warmSurfaces -= 1));
		}
	});
}
