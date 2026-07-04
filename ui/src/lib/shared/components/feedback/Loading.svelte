<script lang="ts">
	import { LoaderCircle } from 'lucide-svelte';
	import { common_loading } from '$lib/paraglide/messages';

	export let message: string | undefined = undefined;

	$: displayMessage = message ?? common_loading();
</script>

<!-- n9e 统一:通用旋转圈(替掉 scanopy 特色 Radar),色随 n9e 主色(blue 已重映射到中海蓝) -->
<div class="py-8 text-center">
	<LoaderCircle class="mx-auto h-8 w-8 animate-spin text-blue-500" />
	<p class="text-secondary mt-2">{displayMessage}</p>
</div>
