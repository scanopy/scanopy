<script lang="ts">
	import type { LogoTheming } from '#lib/shared/utils/logos.ts';

	/**
	 * A logo in the round frame topology cards give it: a ringed disc in the card colour, or in the
	 * logo's own background colour, with the logo inside it. A round logo fills the disc.
	 */
	let {
		size = 24,
		class: className = '',
		iconName,
		iconUrl,
		fill = null,
		round = false,
		theming = 'Fixed',
		darkUrl = null,
		markup = null,
		framed = true
	}: {
		size?: number;
		class?: string;
		iconName: string;
		iconUrl: string;
		/** The colour that fills the frame: the logo's own background. */
		fill?: string | null;
		/** The logo is a disc: drawn at the frame's inner size so it fills it. */
		round?: boolean;
		/**
		 * How it meets the themes: one file (`Fixed`), a black mark and its set's dark-theme file
		 * (`Paired`, `darkUrl`), or `currentColor` markup drawn in the text colour (`TextColor`).
		 */
		theming?: LogoTheming;
		/** The dark-theme file of a `Paired` logo. */
		darkUrl?: string | null;
		/** A `TextColor` logo's SVG, from `logos.json`. */
		markup?: string | null;
		/** False where the surrounding shape is already the frame (a container header): the bare logo. */
		framed?: boolean;
	} = $props();

	// 70% of the frame is the largest square that sits inside the circle, so a square logo fits as
	// well as a round one; the 1px ring takes the rest. Unframed, a logo with a background colour
	// sits on a small rounded square of it, 2px in from each side.
	let logoSize = $derived(
		!framed ? (fill ? size - 4 : size) : round ? size - 2 : Math.round(size * 0.7)
	);
	let paired = $derived(theming === 'Paired' && !!darkUrl);

	const fallbackIcon =
		'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQiIGhlaWdodD0iMjQiIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPGNpcmNsZSBjeD0iMTIiIGN5PSIxMiIgcj0iMTAiIHN0cm9rZT0iY3VycmVudENvbG9yIiBzdHJva2Utd2lkdGg9IjIiLz4KPHA=';

	function handleError(event: Event) {
		(event.currentTarget as HTMLImageElement).src = fallbackIcon;
	}
</script>

<div
	class="inline-flex shrink-0 items-center justify-center {framed
		? 'overflow-hidden rounded-full border border-gray-300 dark:border-gray-600'
		: fill
			? 'rounded-sm'
			: ''} {className}"
	data-logo-fill={fill ?? undefined}
	data-logo-round={round ? '' : undefined}
	style="width: {size}px; height: {size}px; {framed || fill
		? `background: ${fill ?? 'var(--color-topology-node-bg)'};`
		: ''}"
>
	{#if theming === 'TextColor' && markup}
		<!-- Inline, so `currentColor` takes the text colour as the app's own icons do. The markup is
		     a build artifact: generate-fixtures checks it before writing logos.json. -->
		<span
			role="img"
			aria-label="{iconName} icon"
			class="logo-markup block"
			style="width: {logoSize}px; height: {logoSize}px;"
		>
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html markup}
		</span>
	{:else}
		<img
			src={iconUrl}
			alt="{iconName} icon"
			width={logoSize}
			height={logoSize}
			class="block object-contain {paired ? 'dark:hidden' : ''}"
			style="width: {logoSize}px; height: {logoSize}px;"
			onerror={handleError}
		/>
		{#if paired}
			<img
				src={darkUrl}
				alt="{iconName} icon"
				width={logoSize}
				height={logoSize}
				data-logo-dark
				class="hidden object-contain dark:block"
				style="width: {logoSize}px; height: {logoSize}px;"
				onerror={handleError}
			/>
		{/if}
	{/if}
</div>

<style>
	.logo-markup :global(svg) {
		width: 100%;
		height: 100%;
	}
</style>
