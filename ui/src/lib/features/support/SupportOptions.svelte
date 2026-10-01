<script lang="ts">
	import { Bug, BookOpen, Mail } from 'lucide-svelte';
	import { createColorHelper, type Color } from '$lib/shared/utils/styling';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import { supportEmailAddress } from './support';
	import {
		common_email,
		support_discordDesc,
		support_emailDesc,
		support_reportBug,
		support_reportBugDesc,
		support_userGuide
	} from '$lib/paraglide/messages';

	type SupportOption = {
		title: string;
		description: string;
		url: string;
		color: Color;
		icon: IconComponent | string;
	};

	let { isTroubleshooting = false }: { isTroubleshooting?: boolean } = $props();

	const organizationQuery = useOrganizationQuery();
	let supportEmail = $derived.by(() => {
		const planType = organizationQuery.data?.plan?.type;
		return supportEmailAddress(planType ? billingPlans.getMetadata(planType).features : undefined);
	});

	let options = $derived.by(() => {
		if (isTroubleshooting) {
			const items: SupportOption[] = [
				{
					title: support_userGuide(),
					description: 'Daemon setup documentation',
					url: 'https://scanopy.net/docs/setting-up-daemons/',
					color: 'Gray',
					icon: BookOpen
				},
				{
					title: 'Discord',
					description: support_discordDesc(),
					url: 'https://discord.gg/b7ffQr8AcZ',
					color: 'Indigo',
					icon: 'https://cdn.jsdelivr.net/gh/homarr-labs/dashboard-icons/svg/discord.svg'
				},
				{
					title: support_reportBug(),
					description: support_reportBugDesc(),
					url: 'https://github.com/scanopy/scanopy/issues/new?template=bug_report.md',
					color: 'Red',
					icon: Bug
				}
			];

			if (supportEmail) {
				items.push({
					title: common_email(),
					description: support_emailDesc(),
					url: `mailto:${supportEmail}`,
					color: 'Blue',
					icon: Mail
				});
			}

			return items;
		}

		// Full support options — used by SupportModal
		return null;
	});

	function handleCardClick(url: string) {
		window.open(url, '_blank', 'noopener,noreferrer');
	}
</script>

{#if options}
	<div class="grid grid-cols-2 gap-3">
		{#each options as option (option.description)}
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
{/if}
