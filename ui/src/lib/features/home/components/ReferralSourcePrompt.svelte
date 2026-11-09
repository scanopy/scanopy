<script lang="ts">
	import type { components } from '$lib/api/schema';
	import type { PublicServerConfig } from '$lib/shared/stores/config-query';
	import { useReferralSourceMutation } from '$lib/features/organizations/queries';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
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
	} from '$lib/paraglide/messages';

	type Organization = components['schemas']['Organization'];
	type SetupItemType = components['schemas']['SetupItemDiscriminants'];

	let {
		organization,
		configData = null
	}: {
		organization: Organization;
		configData?: PublicServerConfig | null;
	} = $props();

	const setup = $derived(organization.setup ?? []);
	const has = (item: SetupItemType) => setup.includes(item);

	const isCloud = $derived(configData?.deployment_type === 'cloud');

	const visible = $derived(has('InstallDaemon') && !has('AnswerReferralSource') && isCloud);

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
			referralSourceMutation.mutate({
				referral_source: value.referral_source,
				referral_source_other: value.referral_source_other || undefined
			});
		}
	}));

	let selectedSource = $state('');
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			selectedSource = form.state.values.referral_source;
		});
		return unsubscribe;
	});

	function dismiss() {
		// Submit prefer_not_to_say so the milestone is recorded
		referralSourceMutation.mutate({
			referral_source: 'prefer_not_to_say'
		});
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

{#if visible}
	<section>
		<div class="card-static card rounded-lg! p-4!">
			<div class="flex items-center justify-between">
				<div>
					<h3 class="text-sm font-semibold text-primary">{onboarding_howDidYouHear()}</h3>
					<p class="mt-1 text-xs text-secondary">{home_referralSubtitle()}</p>
				</div>
				<button onclick={dismiss} class="text-sm text-tertiary hover:text-secondary">
					{common_dismiss()}
				</button>
			</div>
			<form
				onsubmit={(e) => {
					e.preventDefault();
					e.stopPropagation();
					handleSubmit();
				}}
			>
				<div class="mt-3 grid gap-3" class:sm:grid-cols-2={selectedSource === 'other'}>
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
				<button type="submit" class="btn-primary mt-3 text-sm" disabled={!selectedSource}>
					{common_submit()}
				</button>
			</form>
		</div>
	</section>
{/if}
