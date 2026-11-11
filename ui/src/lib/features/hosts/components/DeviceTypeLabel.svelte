<script lang="ts">
	import type { components } from '#lib/api/schema';
	import { deviceTypes } from '#lib/shared/stores/metadata.ts';

	/** A host's device type as text with its glyph: the hosts table's cell and the editor's fact. */
	let { deviceType }: { deviceType: components['schemas']['DeviceType'] | null | undefined } =
		$props();

	let resolved = $derived(deviceType ?? 'Unidentified');
	let RoleIcon = $derived(deviceTypes.getIconComponent(resolved));
</script>

<span class="inline-flex items-center gap-1.5 text-sm">
	<RoleIcon class="h-4 w-4 shrink-0 {deviceTypes.getColorHelper(resolved).icon}" />
	{deviceTypes.getName(resolved)}
</span>
