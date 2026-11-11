<script lang="ts">
	import CodeContainer from '#lib/shared/components/data/CodeContainer.svelte';
	import { generateEmbedCode, generateEmbedUrl } from '../queries';

	export let shareId: string;

	let width = 800;
	let height = 600;

	$: embedCode = generateEmbedCode(shareId, width, height);
	$: embedUrl = generateEmbedUrl(shareId);
</script>

<div class="space-y-4">
	<div class="flex gap-4">
		<div class="flex-1">
			<label for="embed-width" class="mb-1 block text-sm font-medium text-gray-300">Width</label>
			<input
				type="number"
				id="embed-width"
				bind:value={width}
				min="200"
				max="2000"
				class="input-field"
			/>
		</div>
		<div class="flex-1">
			<label for="embed-height" class="mb-1 block text-sm font-medium text-gray-300">Height</label>
			<input
				type="number"
				id="embed-height"
				bind:value={height}
				min="200"
				max="2000"
				class="input-field"
			/>
		</div>
	</div>

	<div>
		<span class="mb-1 block text-sm font-medium text-gray-300">Embed Code</span>
		<CodeContainer language="html" expandable={false} code={embedCode} />
	</div>

	<div>
		<span class="mb-1 block text-sm font-medium text-gray-300">Direct URL</span>
		<CodeContainer language="bash" expandable={false} code={embedUrl} />
	</div>
</div>
