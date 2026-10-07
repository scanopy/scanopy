<script lang="ts">
	import { Copy } from 'lucide-svelte';
	import { pushSuccess, pushWarning } from '$lib/shared/stores/feedback';
	import { common_clickToCopy, common_copied, common_failedToCopy } from '$lib/paraglide/messages';

	/**
	 * A value the user copies verbatim, drawn as a mono chip that copies on click. `block` is a
	 * shell command on its own line; `inline` is a machine identifier (serial, OID, revision) set
	 * in a row of human text, where the chip marks it as a different kind of value.
	 */
	let { value, variant = 'block' }: { value: string; variant?: 'block' | 'inline' } = $props();

	async function copy() {
		try {
			await navigator.clipboard.writeText(value);
			pushSuccess(common_copied());
		} catch (error) {
			pushWarning(common_failedToCopy({ error: String(error) }));
		}
	}
</script>

<button
	type="button"
	class="group inline-flex max-w-full items-center rounded text-left font-mono text-xs transition-colors {variant ===
	'inline'
		? 'gap-1.5 bg-gray-100 px-1.5 py-0.5 text-gray-800 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700'
		: 'gap-2 bg-gray-900 px-3 py-2 text-gray-300 hover:bg-gray-800 dark:bg-gray-900 dark:hover:bg-gray-800'}"
	onclick={copy}
	title={common_clickToCopy()}
>
	<span class="min-w-0 {variant === 'inline' ? 'break-all' : 'truncate'}">{value}</span>
	<Copy class="h-3 w-3 flex-shrink-0 opacity-0 transition-opacity group-hover:opacity-60" />
</button>
