<script lang="ts">
	// `/pure`, not the default entry. The default one injects the Stripe.js <script> as a
	// top-level side effect of being imported (`Promise.resolve().then(getStripePromise)`), so
	// merely importing this component loads Stripe — and this component is reachable from the
	// root layout via AppShell -> PaymentMethodModal, meaning every page did. On share and embed
	// routes, whose CSP deliberately allows no third-party scripts, that surfaced as a blocked
	// script. `/pure` defers injection until `loadStripe()` is actually called.
	import { loadStripe } from '@stripe/stripe-js/pure';
	// Types only — `/pure` re-exports just the runtime function. `import type` is erased at
	// compile time, so this never pulls the injecting module into the bundle.
	import type { Stripe, StripeElements } from '@stripe/stripe-js';
	import { buildStripeAppearance } from '$lib/shared/billing/stripe-appearance';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import {
		common_cancel,
		common_continue,
		common_processing,
		billing_cardError,
		billing_cardLoadError
	} from '$lib/paraglide/messages';

	let {
		clientSecret,
		description = undefined,
		email = undefined,
		submitLabel = common_continue(),
		onSuccess,
		onCancel = undefined,
		altAction = null
	}: {
		/** Client secret from a backend-created SetupIntent. */
		clientSecret: string;
		/** Optional lead-in text rendered above the Payment Element. */
		description?: string;
		/** Prefills the Link email so returning users skip re-typing it. */
		email?: string;
		submitLabel?: string;
		/**
		 * Called with the confirmed SetupIntent id after the card is collected.
		 * The caller finalizes the payment method (and proceeds, e.g. to
		 * checkout). May be async; the form stays disabled until it resolves.
		 */
		onSuccess: (setupIntentId: string) => void | Promise<void>;
		onCancel?: () => void;
		/**
		 * A different way to pay, offered as a text link under the element.
		 * Stripe's own tabs cover card and bank; anything Stripe does not
		 * collect (invoice billing) belongs here rather than beside them.
		 */
		altAction?: { label: string; onclick: () => void } | null;
	} = $props();

	const configQuery = useConfigQuery();
	let publishableKey = $derived(configQuery.data?.stripe_publishable_key ?? null);
	// Distinguish "config still loading" from "config loaded but key absent" so
	// we can surface a clear error instead of an indefinitely-blank element.
	let configLoaded = $derived(configQuery.data != null);

	let container = $state<HTMLDivElement | null>(null);
	let stripe: Stripe | null = null;
	let elements: StripeElements | null = null;
	let ready = $state(false);
	// Whether the customer has picked a payment method in the element. Nothing
	// to save until they have, so the submit button waits on it.
	let methodSelected = $state(false);
	let busy = $state(false);
	let errorMessage = $state('');
	let loadFailed = $state(false);
	let initialized = false;

	// Mount the Payment Element once we have the publishable key, a client
	// secret, and the container node. loadStripe + element creation happen once.
	$effect(() => {
		if (initialized || !clientSecret || !container) return;

		// Billing is enabled but no publishable key is configured on this
		// deployment — Elements can't load. Fail loudly rather than rendering a
		// blank box. (Operator fix: set SCANOPY_STRIPE_KEY / --stripe-key.)
		if (configLoaded && !publishableKey) {
			initialized = true;
			loadFailed = true;
			errorMessage = billing_cardLoadError();
			console.error(
				'StripeCardForm: stripe_publishable_key missing from /api/config — set SCANOPY_STRIPE_KEY (or --stripe-key) on the server.'
			);
			return;
		}

		if (!publishableKey) return; // config still loading

		initialized = true;
		const node = container;
		void (async () => {
			stripe = await loadStripe(publishableKey);
			if (!stripe) {
				loadFailed = true;
				errorMessage = billing_cardLoadError();
				return;
			}
			// `loader: 'never'` disables Stripe's optimistic skeleton so the user
			// sees our native spinner until the element is fully ready (no flash
			// of placeholder cards). Appearance mirrors the app theme.
			elements = stripe.elements({
				clientSecret,
				appearance: buildStripeAppearance(),
				loader: 'never'
			});
			const paymentElement = elements.create('payment', {
				// `defaultCollapsed` left unset means Stripe expands whichever
				// method it thinks converts best, and then reports that method
				// as selected before the customer has touched anything. Open
				// on the list instead, so "selected" means they picked it.
				layout: { type: 'accordion', defaultCollapsed: true },
				defaultValues: email ? { billingDetails: { email } } : undefined
			});
			paymentElement.on('ready', () => (ready = true));
			// Nothing to save until a method is chosen, so the submit button
			// waits for one.
			paymentElement.on('change', (event) => {
				methodSelected = !event.empty || event.value?.type != null;
			});
			paymentElement.mount(node);
		})();
	});

	async function handleSubmit() {
		if (!stripe || !elements || busy) return;
		busy = true;
		errorMessage = '';

		const { error, setupIntent } = await stripe.confirmSetup({
			elements,
			redirect: 'if_required'
		});

		if (error) {
			errorMessage = error.message ?? billing_cardError();
			busy = false;
			return;
		}

		if (setupIntent?.status === 'succeeded' && setupIntent.id) {
			try {
				await onSuccess(setupIntent.id);
			} catch {
				// onSuccess (finalize/checkout) surfaces its own toast; re-enable
				// so the user can retry. On success the caller unmounts this form.
				busy = false;
			}
			return;
		}

		errorMessage = billing_cardError();
		busy = false;
	}
</script>

<!-- Fills a modal-content flex column: the Payment Element scrolls, the action
     buttons stay pinned in the footer so Continue is always reachable. -->
<form
	onsubmit={(e) => {
		e.preventDefault();
		handleSubmit();
	}}
	class="flex min-h-0 flex-1 flex-col"
>
	<div class="min-h-0 flex-1 space-y-4 overflow-auto p-6">
		{#if description}
			<p class="text-secondary text-sm">{description}</p>
		{/if}

		<!-- Stripe mounts the Payment Element iframe here. Keep it mounted (so it
		     loads) but hidden until `ready`, with our native spinner over it — no
		     flash of Stripe's placeholder cards. -->
		<div class="relative min-h-[8rem]">
			<div bind:this={container} class:invisible={!ready}></div>
			{#if !ready && !loadFailed}
				<div class="absolute inset-0 flex items-center justify-center">
					<Loading />
				</div>
			{/if}
		</div>

		{#if errorMessage}
			<p class="text-sm text-red-400">{errorMessage}</p>
		{/if}

		{#if altAction && ready}
			<div class="text-center">
				<button
					type="button"
					class="text-link text-sm hover:underline"
					disabled={busy}
					onclick={altAction.onclick}
				>
					{altAction.label}
				</button>
			</div>
		{/if}
	</div>

	<div class="modal-footer flex items-center justify-end gap-3">
		{#if onCancel}
			<button type="button" class="btn-secondary" disabled={busy} onclick={onCancel}>
				{common_cancel()}
			</button>
		{/if}
		{#if methodSelected}
			<button type="submit" class="btn-primary" disabled={busy || !ready}>
				{busy ? common_processing() : submitLabel}
			</button>
		{/if}
	</div>
</form>
