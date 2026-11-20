<script lang="ts" module>
	/** A link a translated message places with `slot(key)`. */
	export interface MessageLink {
		href: string;
		label: string;
	}
</script>

<script lang="ts">
	import MessageWithSlots from './MessageWithSlots.svelte';

	let {
		text,
		links = {}
	}: {
		/** A message rendered with `slot(key)` for each parameter that becomes a link. */
		text: string;
		/** The link for each slot key. Links live here, not in the translation, so the
		 *  docs-link test finds and checks every URL. */
		links?: Record<string, MessageLink>;
	} = $props();
</script>

<MessageWithSlots {text}>
	{#snippet part(key)}
		{@const link = links[key]}
		{#if link}<a
				href={link.href}
				target="_blank"
				rel="external noopener noreferrer"
				class="underline hover:no-underline">{link.label}</a
			>{/if}
	{/snippet}
</MessageWithSlots>
