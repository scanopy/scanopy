<script lang="ts">
	import type { components } from '#lib/api/schema';
	import { useReferralSourceMutation } from '#lib/features/organizations/queries.ts';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import SelectInput from '#lib/shared/components/forms/input/SelectInput.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import {
		common_details,
		common_dismiss,
		common_other,
		common_reddit,
		common_source,
		common_submit,
		common_tiktok,
		common_youtube,
		home_referralSubtitle,
		onboarding_howDidYouHear,
		onboarding_referralSource_aiAssistant,
		onboarding_referralSource_blogArticle,
		onboarding_referralSource_hackerNews,
		onboarding_referralSource_otherPlaceholder,
		onboarding_referralSource_preferNotToSay,
		onboarding_referralSource_proxmoxScripts,
		onboarding_referralSource_searchEngine,
		onboarding_referralSource_selfHosted,
		onboarding_referralSource_socialMedia,
		onboarding_referralSource_wordOfMouth
	} from '#lib/paraglide/messages.js';

	/** The AnswerReferralSource item's form. Opened over Home, so it carries no registry name. */
	let { isOpen = false, onClose }: { isOpen?: boolean; onClose: () => void } = $props();

	const referralSourceMutation = useReferralSourceMutation();

	const referralSourceOptions = [
		{ value: '', label: onboarding_howDidYouHear(), disabled: true },
		{ value: 'search_engine', label: onboarding_referralSource_searchEngine() },
		{ value: 'ai_assistant', label: onboarding_referralSource_aiAssistant() },
		{ value: 'youtube', label: common_youtube() },
		{ value: 'tiktok', label: common_tiktok() },
		{ value: 'blog_article', label: onboarding_referralSource_blogArticle() },
		{ value: 'reddit', label: common_reddit() },
		{ value: 'hacker_news', label: onboarding_referralSource_hackerNews() },
		{ value: 'social_media', label: onboarding_referralSource_socialMedia() },
		{ value: 'word_of_mouth', label: onboarding_referralSource_wordOfMouth() },
		{ value: 'proxmox_community_scripts', label: onboarding_referralSource_proxmoxScripts() },
		{ value: 'self_hosted', label: onboarding_referralSource_selfHosted() },
		{ value: 'other', label: common_other() },
		{ value: 'prefer_not_to_say', label: onboarding_referralSource_preferNotToSay() }
	];

	const form = createForm(() => ({
		defaultValues: {
			// `''` is the unselected state, rejected by the guard below before submission.
			referral_source: '' as components['schemas']['ReferralSource'] | '',
			referral_source_other: ''
		},
		onSubmit: async ({ value }) => {
			if (!value.referral_source) return;
			await referralSourceMutation.mutateAsync({
				referral_source: value.referral_source,
				referral_source_other: value.referral_source_other || undefined
			});
			onClose();
		}
	}));

	let selectedSource = $state('');
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			selectedSource = form.state.values.referral_source;
		});
		return unsubscribe;
	});

	/** Records the item as prefer-not-to-say, so it leaves the list. */
	async function dismiss() {
		await referralSourceMutation.mutateAsync({ referral_source: 'prefer_not_to_say' });
		onClose();
	}

	let actions = $derived<ModalAction[]>([
		{ role: 'secondary', label: common_dismiss(), onClick: dismiss },
		{
			role: 'primary',
			label: common_submit(),
			onClick: () => submitForm(form),
			disabled: !selectedSource,
			busy: referralSourceMutation.isPending
		}
	]);
</script>

<GenericModal title={onboarding_howDidYouHear()} {isOpen} {onClose} size="md" {actions}>
	<form
		class="space-y-3 p-6"
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			submitForm(form);
		}}
	>
		<p class="text-sm text-secondary">{home_referralSubtitle()}</p>
		<div class="grid gap-3" class:sm:grid-cols-2={selectedSource === 'other'}>
			<form.Field name="referral_source">
				{#snippet children(field)}
					<SelectInput
						label={common_source()}
						id="referral-source"
						{field}
						options={referralSourceOptions}
					/>
				{/snippet}
			</form.Field>
			{#if selectedSource === 'other'}
				<form.Field name="referral_source_other">
					{#snippet children(field)}
						<TextInput
							label={common_details()}
							id="referral-source-other"
							{field}
							placeholder={onboarding_referralSource_otherPlaceholder()}
						/>
					{/snippet}
				</form.Field>
			{/if}
		</div>
	</form>
</GenericModal>
