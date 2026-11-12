<script lang="ts">
	import { serviceDefinitions } from '#lib/shared/stores/metadata.ts';

	/**
	 * Software: its service definition's logo in a round frame. The circle is the shape the
	 * topology gives every entity whose `glyph` metadata is `Software`. A logo brings its own frame
	 * (`LogoIcon`: the card colour or the logo's background colour, filled by a round logo); a
	 * definition with no logo gets the same ring here around its glyph, on the card's opaque
	 * background, since the dark-mode surface token is 70% translucent.
	 */
	let {
		definition,
		size = 'sm',
		framed = true
	}: {
		definition: string;
		size?: 'sm' | 'lg';
		/** False inside a container header, whose own frame is already the shape. */
		framed?: boolean;
	} = $props();

	let Logo = $derived(serviceDefinitions.getIconComponent(definition));
	let hasLogo = $derived(
		!!(serviceDefinitions.getMetadata(definition) as { logo?: string | null } | null)?.logo
	);
	let framePx = $derived(size === 'lg' ? 48 : 20);
	// A logo's `LogoIcon` takes `framed`; a lucide glyph ignores it. Spread, since the icon type is
	// lucide's and does not declare it.
	const bare = { framed: false };
</script>

{#if !framed}
	<Logo size={16} {...bare} class="text-primary" />
{:else if hasLogo}
	<Logo size={framePx} class="align-top" />
{:else}
	<span
		class="inline-flex shrink-0 items-center justify-center overflow-hidden rounded-full border border-gray-300 align-top dark:border-gray-600 {size ===
		'lg'
			? 'h-12 w-12 border-2'
			: 'h-5 w-5'}"
		style="background: var(--color-topology-node-bg);"
	>
		<!-- No logo: the definition's glyph, drawn in the text colour like a device's so the two
		     read alike. -->
		<Logo size={size === 'lg' ? 28 : 14} class="text-primary" />
	</span>
{/if}
