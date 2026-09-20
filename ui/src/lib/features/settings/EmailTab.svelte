<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useUpdateSelfMutation } from '$lib/features/users/queries';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import { pushSuccess } from '$lib/shared/stores/feedback';
	import type { components } from '$lib/api/schema';
	import {
		settings_email_intro,
		settings_email_digestLabel,
		settings_email_digestDescription,
		settings_email_onboardingLabel,
		settings_email_onboardingDescription,
		settings_email_daemonAlertsLabel,
		settings_email_daemonAlertsDescription,
		settings_email_trialLabel,
		settings_email_trialDescription,
		settings_email_requiredLabel,
		settings_email_requiredDescription,
		settings_email_updated
	} from '$lib/paraglide/messages';

	type EmailSettings = components['schemas']['EmailSettings'];

	const currentUserQuery = useCurrentUserQuery();
	const updateSelfMutation = useUpdateSelfMutation();

	let user = $derived(currentUserQuery.data);

	// Pausable categories default to opted-in — mirrors EmailSettings::default()
	// on the backend. Required emails are not represented here (always sent).
	const DEFAULTS = {
		discovery_digest: true,
		product_onboarding: true,
		daemon_alerts: true,
		trial_and_usage: true
	};

	// Stable literal defaults — do NOT read reactive state inside the createForm
	// options getter (see TanStack + Svelte 5 reactivity notes). `required_emails`
	// is a display-only, always-on toggle and is never persisted.
	const form = createForm(() => ({
		defaultValues: { ...DEFAULTS, required_emails: true },
		onSubmit: async ({ value }) => {
			if (!user) return;
			const email_settings: EmailSettings = {
				discovery_digest: value.discovery_digest,
				product_onboarding: value.product_onboarding,
				daemon_alerts: value.daemon_alerts,
				trial_and_usage: value.trial_and_usage
			};
			try {
				await updateSelfMutation.mutateAsync({ ...user, email_settings });
				pushSuccess(settings_email_updated());
			} catch {
				// Persist failed — revert toggles to the last server value (form.reset is the
				// same path the hydrate uses, so the checkboxes re-render). The API client
				// reports the error. Never leave the UI claiming a state the server rejected.
				resetForm(user.email_settings);
			}
		}
	}));

	// Reset the form without triggering an auto-save — used for both hydration and
	// error-revert, neither of which should fire a write back to the server.
	let suppressSave = false;
	function resetForm(settings: EmailSettings | undefined) {
		suppressSave = true;
		form.reset({
			discovery_digest: settings?.discovery_digest ?? DEFAULTS.discovery_digest,
			product_onboarding: settings?.product_onboarding ?? DEFAULTS.product_onboarding,
			daemon_alerts: settings?.daemon_alerts ?? DEFAULTS.daemon_alerts,
			trial_and_usage: settings?.trial_and_usage ?? DEFAULTS.trial_and_usage,
			required_emails: true
		});
		suppressSave = false;
	}

	// Hydrate the form from server data exactly once when currentUser lands.
	let hydrated = $state(false);
	$effect(() => {
		if (user && !hydrated) {
			resetForm(user.email_settings);
			hydrated = true;
		}
	});

	// Auto-save on each change, debounced so rapid toggles collapse into one request.
	// The checkbox reflects the new value immediately (optimistic) via field.handleChange.
	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	function scheduleSave() {
		if (suppressSave || !hydrated) return;
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => void form.handleSubmit(), 1000);
	}
</script>

<div class="flex flex-1 flex-col">
	<div class="flex-1 overflow-y-auto p-6">
		<div class="space-y-6">
			<p class="text-sm text-gray-600">{settings_email_intro()}</p>

			<InfoCard>
				<div class="space-y-4">
					<form.Field name="discovery_digest" listeners={{ onChange: scheduleSave }}>
						{#snippet children(field)}
							<Checkbox
								id="email-discovery-digest"
								label={settings_email_digestLabel()}
								helpText={settings_email_digestDescription()}
								{field}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="product_onboarding" listeners={{ onChange: scheduleSave }}>
						{#snippet children(field)}
							<Checkbox
								id="email-product-onboarding"
								label={settings_email_onboardingLabel()}
								helpText={settings_email_onboardingDescription()}
								{field}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="daemon_alerts" listeners={{ onChange: scheduleSave }}>
						{#snippet children(field)}
							<Checkbox
								id="email-daemon-alerts"
								label={settings_email_daemonAlertsLabel()}
								helpText={settings_email_daemonAlertsDescription()}
								{field}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="trial_and_usage" listeners={{ onChange: scheduleSave }}>
						{#snippet children(field)}
							<Checkbox
								id="email-trial-and-usage"
								label={settings_email_trialLabel()}
								helpText={settings_email_trialDescription()}
								{field}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="required_emails">
						{#snippet children(field)}
							<Checkbox
								id="email-required"
								label={settings_email_requiredLabel()}
								helpText={settings_email_requiredDescription()}
								disabled
								{field}
							/>
						{/snippet}
					</form.Field>
				</div>
			</InfoCard>
		</div>
	</div>
</div>
