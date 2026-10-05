<script lang="ts" context="module">
	import { entities } from '$lib/shared/stores/metadata';
	import { vlans_vlanLabel } from '$lib/paraglide/messages';

	/**
	 * A VLAN is identified by its number; the name is what a person gave it. A named VLAN shows the
	 * name with the number beneath it, an unnamed one shows the number alone.
	 */
	export const VlanDisplay: EntityDisplayComponent<Vlan, object> = {
		getId: (vlan: Vlan) => vlan.id,
		getLabel: (vlan: Vlan) => vlan.name || vlans_vlanLabel({ number: vlan.vlan_number }),
		getDescription: (vlan: Vlan) =>
			vlan.name ? vlans_vlanLabel({ number: vlan.vlan_number }) : '',
		getIcon: () => entities.getIconComponent('Vlan'),
		getIconColor: () => entities.getColorHelper('Vlan').icon,
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Vlan } from '$lib/features/vlans/types/base';

	export let item: Vlan;
	export let context: object = {};
</script>

<ListSelectItem {item} {context} displayComponent={VlanDisplay} />
