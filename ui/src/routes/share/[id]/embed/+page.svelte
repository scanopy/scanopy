<script lang="ts">
	import { page } from '$app/state';
	import ShareView from '#lib/features/shares/components/ShareView.svelte';

	let shareId = $derived(page.params.id);
</script>

<ShareView {shareId} isEmbed={true} />
