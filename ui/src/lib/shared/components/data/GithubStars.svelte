<script lang="ts">
	import { Icon, Star, type LucideIconData } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import { apiClient } from '#lib/api/client.ts';

	interface Props {
		class?: string;
	}

	let { class: className = '' }: Props = $props();

	// Lucide 1.x dropped its brand icons; this is the GitHub glyph it shipped through 0.x (ISC).
	const github: LucideIconData = {
		name: 'github',
		size: 24,
		node: [
			[
				'path',
				{
					d: 'M15 22v-4a4.8 4.8 0 0 0-1-3.5c3 0 6-2 6-5.5.08-1.25-.27-2.48-1-3.5.28-1.15.28-2.35 0-3.5 0 0-1 0-3 1.5-2.64-.5-5.36-.5-8 0C6 2 5 2 5 2c-.3 1.15-.3 2.35 0 3.5A5.403 5.403 0 0 0 4 9c0 3.5 3 5.5 6 5.5-.39.49-.68 1.05-.85 1.65-.17.6-.22 1.23-.15 1.85v4'
				}
			],
			['path', { d: 'M9 18c-4.51 2-5-2-7-2' }]
		]
	};

	let stars = $state<number | undefined>(undefined);
	let loading = $state(true);
	let error = $state(false);

	async function fetchStars() {
		try {
			const { data } = await apiClient.GET('/api/github-stars', {});
			if (data?.success && typeof data.data === 'number') {
				stars = data.data;
				error = false;
			} else {
				error = true;
			}
		} catch (err) {
			console.error('Error fetching GitHub stars:', err);
			error = true;
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		fetchStars();
	});

	function formatStars(count: number): string {
		if (count >= 1000) {
			return `${(count / 1000).toFixed(1)}k`;
		}
		return count.toString();
	}
</script>

{#if !loading && !error && stars != null && stars != undefined}
	<a
		href="https://github.com/scanopy/scanopy"
		target="_blank"
		rel="noopener noreferrer"
		class="card inline-flex items-center gap-2 rounded-full px-4 py-2 text-sm text-secondary transition-all {className}"
	>
		<Icon icon={github} class="h-4 w-4" />
		<span class="flex items-center gap-1">
			<Star class="h-3.5 w-3.5 fill-yellow-400 text-yellow-400" />
			<span>{formatStars(stars)}</span>
		</span>
	</a>
{:else if loading}
	<div
		class="card inline-flex items-center gap-2 rounded-full px-4 py-2 text-sm text-gray-400 {className}"
	>
		<Icon icon={github} class="h-4 w-4" />
		<span class="flex items-center gap-1">
			<Star class="h-3.5 w-3.5" />
			<span>...</span>
		</span>
	</div>
{/if}
