<script lang="ts">
	import { useProfileUpdateMutation } from '$lib/features/auth/queries';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '$lib/shared/components/layout/modalActions';
	import SelectInput from '$lib/shared/components/forms/input/SelectInput.svelte';
	import {
		common_companySize,
		common_companySize101To250,
		common_companySize1To10,
		common_companySize11To25,
		common_companySize251To500,
		common_companySize26To50,
		common_companySize501To1000,
		common_companySize51To100,
		common_companySizeOver1000,
		common_companySizeSelect,
		common_dismiss,
		common_other,
		common_role,
		common_submit,
		home_profileRoleDevops,
		home_profileRoleExecutive,
		home_profileRoleItAdmin,
		home_profileRoleManager,
		home_profileRoleNetworkEngineer,
		home_profileRoleSelect,
		home_profileSubtitle,
		home_profileTitle
	} from '$lib/paraglide/messages';

	/** The AnswerProfile item's form. Opened over Home, so it carries no registry name. */
	let { isOpen = false, onClose }: { isOpen?: boolean; onClose: () => void } = $props();

	const profileMutation = useProfileUpdateMutation();

	const roleOptions = [
		{ value: '', label: home_profileRoleSelect(), disabled: true },
		{ value: 'it_admin', label: home_profileRoleItAdmin() },
		{ value: 'network_engineer', label: home_profileRoleNetworkEngineer() },
		{ value: 'devops', label: home_profileRoleDevops() },
		{ value: 'manager', label: home_profileRoleManager() },
		{ value: 'executive', label: home_profileRoleExecutive() },
		{ value: 'other', label: common_other() }
	];

	const companySizeOptions = [
		{ value: '', label: common_companySizeSelect(), disabled: true },
		{ value: '1-10', label: common_companySize1To10() },
		{ value: '11-25', label: common_companySize11To25() },
		{ value: '26-50', label: common_companySize26To50() },
		{ value: '51-100', label: common_companySize51To100() },
		{ value: '101-250', label: common_companySize101To250() },
		{ value: '251-500', label: common_companySize251To500() },
		{ value: '501-1000', label: common_companySize501To1000() },
		{ value: '1001+', label: common_companySizeOver1000() }
	];

	const form = createForm(() => ({
		defaultValues: {
			job_title: '',
			company_size: ''
		},
		onSubmit: async ({ value }) => {
			await profileMutation.mutateAsync({
				job_title: value.job_title || undefined,
				company_size: value.company_size || undefined
			});
			onClose();
		}
	}));

	/** Records the item with no answers, so it leaves the list. */
	async function dismiss() {
		await profileMutation.mutateAsync({ job_title: undefined, company_size: undefined });
		onClose();
	}

	let actions = $derived<ModalAction[]>([
		{ role: 'secondary', label: common_dismiss(), onClick: dismiss },
		{
			role: 'primary',
			label: common_submit(),
			onClick: () => submitForm(form),
			busy: profileMutation.isPending
		}
	]);
</script>

<GenericModal title={home_profileTitle()} {isOpen} {onClose} size="md" {actions}>
	<form
		class="space-y-3 p-6"
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			submitForm(form);
		}}
	>
		<p class="text-sm text-secondary">{home_profileSubtitle()}</p>
		<div class="grid gap-3 sm:grid-cols-2">
			<form.Field name="job_title">
				{#snippet children(field)}
					<SelectInput label={common_role()} id="profile-job-title" {field} options={roleOptions} />
				{/snippet}
			</form.Field>
			<form.Field name="company_size">
				{#snippet children(field)}
					<SelectInput
						label={common_companySize()}
						id="profile-company-size"
						{field}
						options={companySizeOptions}
					/>
				{/snippet}
			</form.Field>
		</div>
	</form>
</GenericModal>
