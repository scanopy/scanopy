<script lang="ts">
	// The blurred backdrop behind the sign-in, onboarding and verification
	// screens. Place it first inside a `relative` container.
	import { themeStore } from '#lib/shared/stores/theme.svelte.ts';
</script>

<div class="absolute inset-0 z-0">
	<div
		class="h-full w-full bg-cover bg-center bg-no-repeat blur-xs"
		style="background-image: url('/images/background-{themeStore.resolvedTheme}.webp')"
	></div>
	<div
		class="absolute inset-0 {themeStore.resolvedTheme === 'dark' ? 'bg-black/30' : 'bg-white/15'}"
	></div>
</div>
