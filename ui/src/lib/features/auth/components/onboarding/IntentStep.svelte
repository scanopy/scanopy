<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import type { SignupIntent } from '../../types/base';
	import { onboardingStore } from '../../stores/onboarding';
	import { trackEvent } from '#lib/shared/utils/analytics.ts';
	import { signupIntents } from '#lib/shared/stores/metadata.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { minArrayLength } from '#lib/shared/components/forms/validators.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import { otherAnswer } from './otherAnswer';
	import {
		auth_scanopyLogo,
		common_continue,
		common_skip,
		onboarding_intentsSubtitle,
		onboarding_intentsTitle,
		onboarding_somethingElseLabel,
		onboarding_somethingElsePlaceholder
	} from '#lib/paraglide/messages.js';

	let {
		isOpen,
		onNext,
		onClose
	}: {
		isOpen: boolean;
		onNext: () => void;
		onClose: () => void;
	} = $props();

	const intentIds = signupIntents.getItems().map((item) => item.id as SignupIntent);

	const form = createForm(() => ({
		defaultValues: {
			intents: [...($onboardingStore.intents ?? [])] as SignupIntent[],
			intent_other: $onboardingStore.intentOther ?? ''
		},
		onSubmit: ({ value }) => {
			const other = value.intents.includes('other') ? value.intent_other.trim() || null : null;
			record(value.intents, other, false);
		}
	}));

	// The form's values aren't tracked by `$derived`, so the picked list is mirrored here.
	let picked = $state<SignupIntent[]>([...($onboardingStore.intents ?? [])]);
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			picked = form.state.values.intents;
		});
		return unsubscribe;
	});

	function toggle(intent: SignupIntent) {
		const next = picked.includes(intent)
			? picked.filter((i) => i !== intent)
			: intentIds.filter((i) => i === intent || picked.includes(i));
		form.setFieldValue('intents', next);
	}

	function record(intents: SignupIntent[], other: string | null, skipped: boolean) {
		trackEvent('onboarding_intents_selected', {
			intents,
			skipped,
			hosting: onboardingStore.getState().hosting
		});
		onboardingStore.setIntents(intents, other);
		onNext();
	}

	let actions = $derived<ModalAction[]>([
		{ role: 'secondary', label: common_skip(), onClick: () => record([], null, true) },
		{
			role: 'primary',
			label: common_continue(),
			onClick: () => submitForm(form),
			disabled: picked.length === 0
		}
	]);
</script>

<GenericModal
	{isOpen}
	title={onboarding_intentsTitle()}
	{onClose}
	size="lg"
	centerTitle={true}
	showBackdrop={false}
	showCloseButton={false}
	preventCloseOnClickOutside={true}
	{actions}
	footerLayout="stack"
>
	{#snippet headerIcon()}
		<img src="/logos/scanopy-logo.png" alt={auth_scanopyLogo()} class="h-8 w-8" />
	{/snippet}

	<form
		class="flex min-h-0 flex-1 flex-col"
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			submitForm(form);
		}}
	>
		<div class="flex-1 space-y-4 overflow-y-auto p-6">
			<p class="text-center text-sm text-secondary">{onboarding_intentsSubtitle()}</p>
			<form.Field name="intents" validators={{ onSubmit: ({ value }) => minArrayLength(1)(value) }}>
				{#snippet children()}
					<div class="grid gap-2">
						{#each intentIds as intentId (intentId)}
							{@const checked = picked.includes(intentId)}
							{@const Icon = signupIntents.getIconComponent(intentId)}
							<div class="card-static card p-0 {checked ? 'card-selected' : ''}">
								<label class="flex cursor-pointer items-center gap-3 p-3">
									<input
										type="checkbox"
										class="checkbox-card h-4 w-4"
										{checked}
										onchange={() => toggle(intentId)}
									/>
									<Icon class="h-4 w-4 shrink-0 text-secondary" />
									<span class="text-sm text-primary">{signupIntents.getName(intentId)}</span>
								</label>
								{#if checked && intentId === 'other'}
									<div class="px-3 pb-3">
										<form.Field name="intent_other" validators={{ onSubmit: otherAnswer }}>
											{#snippet children(field)}
												<TextInput
													label={onboarding_somethingElseLabel()}
													id="intent-other"
													{field}
													placeholder={onboarding_somethingElsePlaceholder()}
													autofocus
												/>
											{/snippet}
										</form.Field>
									</div>
								{/if}
							</div>
						{/each}
					</div>
				{/snippet}
			</form.Field>
		</div>
	</form>
</GenericModal>
