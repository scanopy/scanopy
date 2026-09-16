<script lang="ts">
	import { Monitor, Sun, Moon } from 'lucide-svelte';
	import { themeStore } from '$lib/shared/stores/theme.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		common_system,
		common_theme,
		common_light,
		common_dark,
		common_source,
		settings_system_copyright,
		settings_system_license,
		settings_system_licenseLinkText,
		settings_system_themeDesc,
		settings_system_tuxAttribution
	} from '$lib/paraglide/messages';

	const options = [
		{ id: 'system' as const, label: common_system(), icon: Monitor },
		{ id: 'light' as const, label: common_light(), icon: Sun },
		{ id: 'dark' as const, label: common_dark(), icon: Moon }
	];

	const copyrightYear = new Date().getFullYear();
</script>

<div class="flex h-full flex-col gap-6 overflow-y-auto p-6">
	<div>
		<h3 class="text-primary text-sm font-semibold">{common_theme()}</h3>
		<p class="text-tertiary mt-1 text-sm">{settings_system_themeDesc()}</p>
		<div class="mt-2 flex gap-2">
			{#each options as option (option.id)}
				<button
					type="button"
					class="btn-secondary flex flex-1 items-center justify-center gap-1.5 {themeStore.themeMode ===
					option.id
						? 'ring-primary ring-2'
						: ''}"
					onclick={() => themeStore.setTheme(option.id)}
				>
					<option.icon size={16} />
					{option.label}
				</button>
			{/each}
		</div>
	</div>

	<div class="mt-auto flex flex-col gap-1">
		<p class="text-tertiary text-xs">{settings_system_copyright({ year: copyrightYear })}</p>
		<DocsHint
			text={settings_system_license()}
			href="https://github.com/scanopy/scanopy/blob/main/COMMERCIAL-LICENSE.md"
			linkText={settings_system_licenseLinkText()}
		/>
		<DocsHint
			text={settings_system_tuxAttribution()}
			href="https://commons.wikimedia.org/wiki/File:Tux.svg"
			linkText={common_source()}
		/>
	</div>
</div>
