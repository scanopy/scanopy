<script lang="ts">
	import {
		Bug,
		AlertTriangle,
		Lightbulb,
		BookOpen,
		Mail,
		CalendarCheck,
		MessageCircle,
		FlaskConical
	} from '@lucide/svelte';
	import { logoPath } from '#lib/shared/utils/logos.ts';
	import { createColorHelper, type Color } from '#lib/shared/utils/styling.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { billingPlans, features } from '#lib/shared/stores/metadata.ts';
	import { isCloud, useConfigQuery } from '#lib/shared/stores/config-query.ts';
	import {
		BOOK_CALL_URL,
		betaProgramUrl,
		supportEmailAddress
	} from '#lib/features/support/support.ts';
	import {
		common_email,
		support_betaProgram,
		support_betaProgramDesc,
		support_betaProgramFeaturesDesc,
		support_discordDesc,
		support_emailDesc,
		support_incorrectDetection,
		support_incorrectDetectionDesc,
		support_onboardingCall,
		support_onboardingCallDesc,
		support_priorityEmailDesc,
		support_reportBug,
		support_reportBugDesc,
		support_requestFeature,
		support_requestFeatureDesc,
		support_userGuide,
		support_userGuideDesc
	} from '#lib/paraglide/messages.js';

	type SupportOption = {
		title: string;
		description: string;
		url: string;
		color: Color;
		icon: IconComponent | string;
	};

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let planFeatures = $derived(
		organization?.plan ? billingPlans.getMetadata(organization.plan.type).features : undefined
	);
	let supportEmail = $derived(supportEmailAddress(planFeatures));

	const configQuery = useConfigQuery();
	let cloud = $derived(configQuery.data ? isCloud(configQuery.data) : false);

	// The features open to beta program members first, named on the beta program card.
	let betaFeatureNames = $derived(
		(features.getItems() ?? [])
			.filter((f) => f.metadata?.stability === 'Beta')
			.map((f) => features.getName(f.id))
	);

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
				icon: logoPath('Discord') ?? MessageCircle
			},
			{
				title: support_betaProgram(),
				description:
					betaFeatureNames.length > 0
						? support_betaProgramFeaturesDesc({ features: betaFeatureNames.join(', ') })
						: support_betaProgramDesc(),
				url: betaProgramUrl('app_settings', cloud),
				color: 'Purple',
				icon: FlaskConical
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

<div class="flex min-h-0 flex-1 flex-col">
	<div class="flex-1 space-y-6 overflow-auto p-6">
		<div class="grid grid-cols-1 gap-3 sm:grid-cols-2">
			{#each supportOptions as option (option.description)}
				{@const colors = createColorHelper(option.color)}
				<button onclick={() => handleCardClick(option.url)} class="card w-full text-left">
					<div class="flex items-center gap-3">
						<div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {colors.bg}">
							{#if typeof option.icon === 'string'}
								<img src={option.icon} alt={option.title} class="h-5 w-5 object-contain" />
							{:else}
								<option.icon class="h-5 w-5 {colors.icon}" />
							{/if}
						</div>
						<div class="min-w-0 flex-1">
							<p class="text-sm font-medium text-primary">{option.title}</p>
							<p class="truncate text-xs text-secondary">{option.description}</p>
						</div>
					</div>
				</button>
			{/each}
		</div>
	</div>
</div>
