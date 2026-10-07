<script lang="ts">
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import {
		topology_shortcutsTitle,
		topology_shortcutSearch,
		topology_shortcutFitView,
		topology_shortcutZoomSelection,
		topology_shortcutDeselect,
		topology_shortcutHelp,
		topology_shortcutBoxSelect,
		topology_shortcutToggleSelect,
		topology_shortcutStepExpand,
		topology_shortcutStepCollapse
	} from '$lib/paraglide/messages';
	import KbdKey from '$lib/shared/components/feedback/KbdKey.svelte';
	import { keyLabel } from '$lib/shared/utils/shortcuts';

	let { isOpen = $bindable(false), readonly = false }: { isOpen: boolean; readonly?: boolean } =
		$props();

	// Keys are named as `keyLabel` takes them, so each chip shows this platform's spelling. `appOnly`
	// rows are left off shared views, where multi-select (which drives dependency editing) is off.
	const allShortcuts = [
		{ keys: ['Mod', 'F'], description: () => topology_shortcutSearch() },
		{ keys: ['F'], description: () => topology_shortcutFitView() },
		{ keys: ['Z'], description: () => topology_shortcutZoomSelection() },
		{ keys: [']'], description: () => topology_shortcutStepExpand() },
		{ keys: ['['], description: () => topology_shortcutStepCollapse() },
		{ keys: ['Shift', 'Drag'], description: () => topology_shortcutBoxSelect() },
		{ keys: ['Mod', 'Click'], description: () => topology_shortcutToggleSelect(), appOnly: true },
		{ keys: ['Escape'], description: () => topology_shortcutDeselect() },
		{ keys: ['?'], description: () => topology_shortcutHelp() }
	];

	let shortcuts = $derived(readonly ? allShortcuts.filter((s) => !s.appOnly) : allShortcuts);
</script>

<GenericModal title={topology_shortcutsTitle()} {isOpen} onClose={() => (isOpen = false)} size="sm">
	<div class="space-y-1 p-4">
		{#each shortcuts as shortcut (shortcut.keys.join('+'))}
			<div class="flex items-center justify-between py-1.5">
				<span class="text-secondary text-sm">{shortcut.description()}</span>
				<div class="flex items-center gap-1">
					{#each shortcut.keys as key (key)}
						<KbdKey key={keyLabel(key)} size="md" />
					{/each}
				</div>
			</div>
		{/each}
	</div>
</GenericModal>
