<script lang="ts">
	import { ChevronDown } from 'lucide-svelte';
	import CreatableOptionList, {
		type CreatableOption
	} from '$lib/shared/components/forms/selection/CreatableOptionList.svelte';
	import { concepts } from '$lib/shared/stores/metadata';
	import { exclusiveSetLabel, type ExclusiveSet } from '../sets';
	import { common_application, common_none, tags_createTagQuoted } from '$lib/paraglide/messages';

	/**
	 * Choose the exclusive set a tag belongs to: the built-in Application set, a set another tag
	 * already names, a new set typed here, or none. A tag sits in at most one set, so this is a
	 * single value rather than a list.
	 */
	let {
		value,
		groups,
		onChange,
		id,
		placeholder,
		disabled = false
	}: {
		value: ExclusiveSet | null;
		/** Names of the sets already in use. */
		groups: string[];
		onChange: (set: ExclusiveSet | null) => void;
		id: string;
		placeholder: string;
		disabled?: boolean;
	} = $props();

	const NONE = 'none';
	const APPLICATION = 'application';
	const GROUP_PREFIX = 'group:';

	let open = $state(false);
	let query = $state('');

	let needle = $derived(query.trim().toLowerCase());
	let matchingGroups = $derived(groups.filter((name) => name.toLowerCase().includes(needle)));

	let options = $derived.by<CreatableOption[]>(() => {
		const result: CreatableOption[] = [];
		if (value && !needle) result.push({ id: NONE, label: common_none() });
		if (common_application().toLowerCase().includes(needle)) {
			result.push({
				id: APPLICATION,
				label: common_application(),
				icon: concepts.getIconComponent('Application')
			});
		}
		for (const name of matchingGroups) result.push({ id: GROUP_PREFIX + name, label: name });
		return result;
	});

	// Offer to create what was typed unless it already names a set.
	let canCreate = $derived(
		needle.length > 0 &&
			!groups.some((name) => name.toLowerCase() === needle) &&
			common_application().toLowerCase() !== needle
	);

	function select(optionId: string) {
		if (optionId === NONE) onChange(null);
		else if (optionId === APPLICATION) onChange({ type: 'Application' });
		else onChange({ type: 'Group', name: optionId.slice(GROUP_PREFIX.length) });
		close();
	}

	function create() {
		const name = query.trim();
		if (!name) return;
		onChange({ type: 'Group', name });
		close();
	}

	function close() {
		open = false;
		query = '';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			if (options.length > 0) select(options[0].id);
			else if (canCreate) create();
		} else if (e.key === 'Escape') {
			close();
		}
	}
</script>

<div class="relative">
	<input
		{id}
		type="text"
		class="input-field w-full pr-8"
		{disabled}
		placeholder={open ? '' : placeholder}
		value={open ? query : (exclusiveSetLabel(value) ?? '')}
		onfocus={() => (open = true)}
		onblur={() => setTimeout(close, 150)}
		oninput={(e) => (query = e.currentTarget.value)}
		onkeydown={handleKeydown}
		autocomplete="off"
	/>
	<ChevronDown
		class="text-tertiary pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2"
	/>

	{#if open && (options.length > 0 || canCreate)}
		<div
			class="select-dropdown absolute left-0 right-0 z-50 mt-1 max-h-48 overflow-y-auto rounded-md shadow-lg"
		>
			<CreatableOptionList
				{options}
				onSelect={select}
				createLabel={canCreate ? tags_createTagQuoted({ name: query.trim() }) : null}
				onCreate={create}
			/>
		</div>
	{/if}
</div>
