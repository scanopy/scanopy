<script lang="ts" generics="T, C">
	import ListSelectItem from '#lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';

	export let item: T;
	export let context: C;
	export let displayComponent: EntityDisplayComponent<T, C>;
</script>

<ListSelectItem {item} {context} {displayComponent} />
