<script lang="ts">
	import { ArrowUpCircle } from '@lucide/svelte';
	import AppBanner from './AppBanner.svelte';
	import { docsUrl } from '#lib/shared/utils/docs.ts';
	import { VERSION } from '#lib/version.ts';
	import {
		upgradeBanner_body,
		upgradeBanner_howToUpgrade,
		common_releaseNotes
	} from '#lib/paraglide/messages.js';
	import type { PublicServerConfig } from '#lib/shared/stores/config-query.ts';

	let { upgrade }: { upgrade: NonNullable<PublicServerConfig['available_upgrade']> } = $props();

	function withUtm(url: string, content: 'release_notes' | 'upgrade_docs'): string {
		const params = new URLSearchParams({
			utm_source: 'app',
			utm_medium: 'in_app',
			utm_campaign: 'upgrade_available',
			utm_content: content
		});
		return `${url}?${params.toString()}`;
	}
</script>

<!-- Keyed per version: dismissing hides this release, and the next one shows again. -->
<AppBanner
	variant="info"
	icon={ArrowUpCircle}
	body={upgradeBanner_body({ latest: upgrade.version, current: VERSION })}
	dismissableKey={`upgrade_available:v${upgrade.version}`}
>
	{#snippet actions()}
		<!-- eslint-disable svelte/no-navigation-without-resolve -->
		<a
			href={withUtm(upgrade.release_notes_url, 'release_notes')}
			target="_blank"
			rel="noopener noreferrer"
			class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline"
		>
			{common_releaseNotes()}
		</a>
		<a
			href={withUtm(docsUrl('/docs/self-hosted-server/upgrading/'), 'upgrade_docs')}
			target="_blank"
			rel="noopener noreferrer"
			class="ml-2 rounded-sm px-2 py-0.5 text-xs font-medium underline hover:no-underline"
		>
			{upgradeBanner_howToUpgrade()}
		</a>
		<!-- eslint-enable svelte/no-navigation-without-resolve -->
	{/snippet}
</AppBanner>
