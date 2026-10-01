<script lang="ts">
	import {
		Bug,
		AlertTriangle,
		Lightbulb,
		LifeBuoy,
		BookOpen,
		Mail,
		CalendarCheck
	} from 'lucide-svelte';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import { VERSION } from '$lib/version';
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { useOrganizationQuery } from '../organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import { BOOK_CALL_URL, supportEmailAddress } from './support';
	import {
		common_close,
		common_email,
		common_userId,
		common_version,
		support_description,
		support_discordDesc,
		support_emailDesc,
		support_incorrectDetection,
		support_incorrectDetectionDesc,
		support_info,
		support_onboardingCall,
		support_onboardingCallDesc,
		support_orgId,
		support_priorityEmailDesc,
		support_reportBug,
		support_reportBugDesc,
		support_requestFeature,
		support_requestFeatureDesc,
		support_title,
		support_userGuide,
		support_userGuideDesc
	} from '$lib/paraglide/messages';

	type SupportOption = {
		title: string;
		description: string;
		url: string;
		color: Color;
		icon: IconComponent | string;
	};

	let {
		isOpen = false,
		onClose,
		name = undefined
	}: {
		isOpen: boolean;
		onClose: () => void;
		name?: string;
	} = $props();

	// TanStack Query for current user and organization
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let planFeatures = $derived(
		organization?.plan ? billingPlans.getMetadata(organization.plan.type).features : undefined
	);
	let supportEmail = $derived(supportEmailAddress(planFeatures));

	let supportOptions = $derived.by(() => {
		const options: SupportOption[] = [
			{
				title: support_userGuide(),
				description: support_userGuideDesc(),
				url: 'https://scanopy.net/docs/',
				color: 'Gray',
				icon: BookOpen
			},
			{
				title: support_incorrectDetection(),
				description: support_incorrectDetectionDesc(),
				url: 'https://github.com/scanopy/scanopy/issues/new?template=service-detection-issue.md',
				color: 'Yellow',
				icon: AlertTriangle
			},
			{
				title: support_requestFeature(),
				description: support_requestFeatureDesc(),
				url: 'https://github.com/scanopy/scanopy/issues/new?template=feature_request.md',
				color: 'Green',
				icon: Lightbulb
			},
			{
				title: support_reportBug(),
				description: support_reportBugDesc(),
				url: 'https://github.com/scanopy/scanopy/issues/new?template=bug_report.md',
				color: 'Red',
				icon: Bug
			},
			{
				title: 'Discord',
				description: support_discordDesc(),
				url: 'https://discord.gg/b7ffQr8AcZ',
				color: 'Indigo',
				icon: 'https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/discord.svg'
			}
		];

		if (supportEmail) {
			options.push({
				title: common_email(),
				description: planFeatures?.priority_support
					? support_priorityEmailDesc()
					: support_emailDesc(),
				url: `mailto:${supportEmail}`,
				color: 'Blue',
				icon: Mail
			});
		}

		if (planFeatures?.onboarding_call) {
			options.push({
				title: support_onboardingCall(),
				description: support_onboardingCallDesc(),
				url: BOOK_CALL_URL,
				color: 'Teal',
				icon: CalendarCheck
			});
		}

		return options;
	});

	function handleCardClick(url: string) {
		window.open(url, '_blank', 'noopener,noreferrer');
	}
</script>

<GenericModal {isOpen} title={support_title()} {name} {onClose} size="xl">
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={LifeBuoy} color="Blue" />
	{/snippet}

	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 space-y-6 overflow-auto p-6">
			<p class="text-secondary text-sm">
				{support_description()}
			</p>

			<div class="grid grid-cols-2 gap-3">
				{#each supportOptions as option (option.description)}
					{@const colors = createColorHelper(option.color)}
					<button onclick={() => handleCardClick(option.url)} class="card w-full text-left">
						<div class="flex items-center gap-3">
							<div
								class="flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-lg {colors.bg}"
							>
								{#if typeof option.icon === 'string'}
									<img src={option.icon} alt={option.title} class="h-5 w-5" />
								{:else}
									<option.icon class="h-5 w-5 {colors.icon}" />
								{/if}
							</div>
							<div class="min-w-0 flex-1">
								<p class="text-primary text-sm font-medium">{option.title}</p>
								<p class="text-secondary truncate text-xs">{option.description}</p>
							</div>
						</div>
					</button>
				{/each}
			</div>
			<InfoCard title={support_info()}>
				<InfoRow label={common_version()}>{VERSION}</InfoRow>
				<InfoRow label={support_orgId()} mono={true}>
					{organization?.id ?? '—'}
				</InfoRow>
				<InfoRow label={common_userId()} mono={true}>{currentUser?.id ?? '—'}</InfoRow>
			</InfoCard>
		</div>

		<div class="modal-footer">
			<div class="flex justify-end">
				<button type="button" onclick={onClose} class="btn-secondary">{common_close()}</button>
			</div>
		</div>
	</div>
</GenericModal>
