<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import type { components } from '$lib/api/schema';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required } from '$lib/shared/components/forms/validators';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import { CheckCircle } from 'lucide-svelte';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		billing_inquiry1To3Months,
		billing_inquiry3To6Months,
		billing_inquiryIntro,
		billing_inquiryJustExploring,
		billing_inquiryMessagePlaceholder,
		billing_inquiryNamePlaceholder,
		billing_inquiryNetworkCountLabel,
		billing_inquiryNetworkCountPlaceholder,
		billing_inquiryReceived,
		billing_inquiryTeamSizeSelect,
		billing_inquiryThanks,
		billing_inquiryTimelineLabel,
		billing_inquiryTimelineSelect,
		billing_requestInfo,
		common_cancel,
		common_close,
		common_companySize,
		common_companySize101To250,
		common_companySize1To10,
		common_companySize11To25,
		common_companySize251To500,
		common_companySize26To50,
		common_companySize501To1000,
		common_companySize51To100,
		common_companySizeOver1000,
		common_immediately,
		common_message,
		common_name,
		common_sending,
		common_somethingWentWrong,
		common_submit
	} from '$lib/paraglide/messages';
	import { apiClient } from '$lib/api/client';
	import { requireSuccess } from '$lib/api/query-helpers';

	interface Props {
		isOpen?: boolean;
		planName?: string;
		planType?: string;
		userEmail?: string;
		orgName?: string;
		companySize?: string;
		onClose: () => void;
	}

	let {
		isOpen = false,
		planName = '',
		planType = '',
		userEmail = '',
		orgName = '',
		companySize = '',
		onClose
	}: Props = $props();

	let loading = $state(false);
	let status = $state<'idle' | 'success' | 'error'>('idle');
	let submitError = $state('');

	const teamSizeOptions = [
		{ value: '', label: billing_inquiryTeamSizeSelect(), disabled: true },
		{ value: '1-10', label: common_companySize1To10() },
		{ value: '11-25', label: common_companySize11To25() },
		{ value: '26-50', label: common_companySize26To50() },
		{ value: '51-100', label: common_companySize51To100() },
		{ value: '101-250', label: common_companySize101To250() },
		{ value: '251-500', label: common_companySize251To500() },
		{ value: '501-1000', label: common_companySize501To1000() },
		{ value: '1001+', label: common_companySizeOver1000() }
	];

	const urgencyOptions = [
		{ value: '', label: billing_inquiryTimelineSelect(), disabled: true },
		{ value: 'immediately', label: common_immediately() },
		{ value: '1-3 months', label: billing_inquiry1To3Months() },
		{ value: '3-6 months', label: billing_inquiry3To6Months() },
		{ value: 'exploring', label: billing_inquiryJustExploring() }
	];

	// `''` is the placeholder option, which the field's `required` validator rejects before
	// submission — hence the guard at the submit site rather than a cast.
	type TeamSize = components['schemas']['TeamSize'];
	type InquiryTimeline = components['schemas']['InquiryTimeline'];

	function getDefaultValues() {
		return {
			name: '',
			teamSize: companySize as TeamSize | '',
			message: '',
			urgency: '' as InquiryTimeline | '',
			networkCount: undefined as number | undefined
		};
	}

	const form = createForm(() => ({
		defaultValues: getDefaultValues(),
		onSubmit: async ({ value }) => {
			if (!value.teamSize) {
				return;
			}
			const teamSize = value.teamSize;

			loading = true;
			submitError = '';

			try {
				requireSuccess(
					await apiClient.POST('/api/billing/inquiry', {
						body: {
							email: userEmail,
							name: value.name.trim(),
							company: orgName,
							team_size: teamSize,
							message: value.message.trim(),
							urgency: value.urgency || undefined,
							network_count: value.networkCount ?? undefined,
							plan_type: planType || undefined
						}
					})
				);

				status = 'success';
				trackEvent('plan_inquiry_submitted', { planType, success: true });
			} catch (err) {
				console.error('Plan inquiry form error:', err);
				submitError = common_somethingWentWrong();
				trackEvent('plan_inquiry_submitted', { planType, success: false });
			} finally {
				loading = false;
			}
		}
	}));

	function handleOpen() {
		form.reset(getDefaultValues());
		status = 'idle';
		submitError = '';
	}

	function handleClose() {
		status = 'idle';
		submitError = '';
		onClose();
	}

	async function handleSubmit() {
		await submitForm(form);
	}
</script>

<GenericModal
	title={billing_requestInfo({ planName })}
	{isOpen}
	onClose={handleClose}
	onOpen={handleOpen}
	size="md"
	showCloseButton={true}
>
	{#if status === 'success'}
		<div class="flex flex-col items-center justify-center p-8 text-center">
			<div class="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-500/20">
				<CheckCircle class="h-8 w-8 text-green-400" />
			</div>
			<h3 class="text-primary mb-2 text-xl font-semibold">{billing_inquiryThanks()}</h3>
			<p class="text-secondary mb-6">
				{billing_inquiryReceived({ planName })}
			</p>
			<button type="button" onclick={handleClose} class="btn-primary">{common_close()}</button>
		</div>
	{:else}
		<form
			onsubmit={(e) => {
				e.preventDefault();
				e.stopPropagation();
				handleSubmit();
			}}
			class="flex min-h-0 flex-1 flex-col"
		>
			<div class="flex-1 overflow-auto p-6">
				<p class="text-secondary mb-6 text-sm">
					{billing_inquiryIntro()}
				</p>

				<div class="space-y-4">
					<form.Field
						name="name"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<TextInput
								label={common_name()}
								id="inquiry-name"
								{field}
								placeholder={billing_inquiryNamePlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="teamSize"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<SelectInput
								label={common_companySize()}
								id="inquiry-team-size"
								{field}
								options={teamSizeOptions}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field
						name="urgency"
						validators={{
							onBlur: ({ value }) => required(value)
						}}
					>
						{#snippet children(field)}
							<SelectInput
								label={billing_inquiryTimelineLabel()}
								id="inquiry-urgency"
								{field}
								options={urgencyOptions}
								required
							/>
						{/snippet}
					</form.Field>

					<form.Field name="networkCount">
						{#snippet children(field)}
							<TextInput
								label={billing_inquiryNetworkCountLabel()}
								id="inquiry-network-count"
								{field}
								type="number"
								placeholder={billing_inquiryNetworkCountPlaceholder()}
							/>
						{/snippet}
					</form.Field>

					<form.Field name="message">
						{#snippet children(field)}
							<TextArea
								label={common_message()}
								id="inquiry-message"
								{field}
								placeholder={billing_inquiryMessagePlaceholder()}
								rows={3}
							/>
						{/snippet}
					</form.Field>

					{#if submitError}
						<p class="text-sm text-red-400">{submitError}</p>
					{/if}
				</div>
			</div>

			<div class="modal-footer">
				<div class="flex items-center justify-end gap-3">
					<button type="button" disabled={loading} onclick={handleClose} class="btn-secondary">
						{common_cancel()}
					</button>
					<button type="submit" disabled={loading} class="btn-primary">
						{loading ? common_sending() : common_submit()}
					</button>
				</div>
			</div>
		</form>
	{/if}
</GenericModal>
