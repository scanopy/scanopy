<script lang="ts">
	import { Bug, BookOpen, Mail, MessageCircle } from '@lucide/svelte';
	import { logoPath } from '#lib/shared/utils/logos.ts';
	import { createColorHelper, type Color } from '#lib/shared/utils/styling.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { billingPlans } from '#lib/shared/stores/metadata.ts';
	import { BUG_REPORT_URL, supportEmailAddress } from './support';
	import {
		common_email,
		support_discordDesc,
		support_emailDesc,
		support_reportBug,
		support_reportBugDesc,
		support_userGuide
	} from '#lib/paraglide/messages.js';

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
					icon: logoPath('Discord') ?? MessageCircle
				},
				{
					title: support_reportBug(),
					description: support_reportBugDesc(),
					url: BUG_REPORT_URL,
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

		// The full list lives in Settings → Support (settings/SupportTab.svelte)
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
{/if}
