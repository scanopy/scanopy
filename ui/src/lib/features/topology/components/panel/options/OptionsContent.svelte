<script lang="ts">
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { FunnelX } from 'lucide-svelte';
	import type { components } from '$lib/api/schema';
	import {
		topologyOptions,
		updateTopologyOptions,
		selectedTopologyId,
		useTopologiesQuery
	} from '../../../queries';
	import { hoveredEdgeType, presentFilterValues } from '../../../interactions';
	import { isDisabledEdge } from '../../../layout/edge-classification';
	import { useTopology } from '../../../context';
	import { getTopologyEditState, getOptionDisabledTooltip } from '../../../state';
	import { edgeTypes, views } from '$lib/shared/stores/metadata';
	import { activeView, clearedHideSetFor, declaredMetadataFiltersFor } from '../../../queries';
	import { type Color } from '$lib/shared/utils/styling';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import type { RenderableTopology } from '../../../types/base';
	import viewsJson from '$lib/data/views.json';
	import TagFilterGroup from './TagFilterGroup.svelte';
	import OptionToggle from './OptionToggle.svelte';
	import CategoryFilterGroup from './CategoryFilterGroup.svelte';
	import FilterGroup from './FilterGroup.svelte';
	import GroupingRuleEditor from './GroupingRuleEditor.svelte';
	import EntityFilterHeader from './EntityFilterHeader.svelte';
	import { useTagsQuery } from '$lib/features/tags/queries';
	import {
		common_visual,
		common_dependenciesLabel,
		topology_bundleEdges,
		topology_bundleEdgesHelp,
		topology_dontFadeEdges,
		topology_dontFadeEdgesHelp,
		topology_showMinimap,
		topology_showMinimapHelp,
		common_byTag,
		common_edges,
		common_clearAll,
		topology_filtersApplyToView,
		topology_groupsHelp,
		topology_displayHelp,
		topology_nFiltersApplied
	} from '$lib/paraglide/messages';

	type EntityType = components['schemas']['EntityDiscriminants'];

	let {
		activeTab,
		renderableTopology
	}: {
		activeTab: 'filter' | 'group' | 'visual';
		/** Enriched bundle for the active view — network- and snapshot-scoped. */
		renderableTopology: RenderableTopology | undefined;
	} = $props();

	// Get topology row for layout/edit state.
	const topologiesQuery = useTopologiesQuery();
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let topology = $derived(topologiesData.find((t) => t.id === $selectedTopologyId));

	// The enriched topology (from context) carries the active view's built
	// graph; the row no longer stores nodes/edges.
	const topo = useTopology();
	const renderStore = topo.fromContext ? topo.store : null;
	let renderTopology = $derived(renderStore ? $renderStore : null);

	// Unified edit state for gating request-path options
	// Display filters (category / visibility) apply client-side via
	// `tagHiddenNodeIds`, so they work in a read-only snapshot just like the tag
	// and edge-type filters — they don't persist (`saveOptions` bails when
	// read-only) and don't need a rebuild. Read-only enforcement that matters
	// (grouping rules, which DO need a rebuild) lives in `GroupingRuleEditor`.
	let editState = $derived(getTopologyEditState(topology, false, false));

	// Live entity arrays drive tag-filter sections. Hosts query populates the
	// services cache as a side-effect; subnets fetch directly.
	// Host tags come from the topology bundle rather than a separate hosts fetch.
	//
	// This used to be `useHostsQuery({ limit: 0 })` — every host in the
	// organisation, unpaginated (~1.9MB on a 440-host estate), downloaded and
	// parsed on the critical path to interactive, to produce a set of tag ids and
	// one boolean.
	//
	// It also disagreed with the graph. That query passed neither `network_id`
	// nor `at`, so the filter offered tags from hosts on networks you were not
	// viewing, and in snapshot mode offered *live* host tags while the graph
	// showed the snapshot. The bundle is already scoped to the selected network
	// and snapshot, so reading from it makes the filter match what is on screen.
	const servicesCacheQuery = useServicesCacheQuery();
	const subnetsQuery = useSubnetsQuery();
	let hostsData = $derived(renderableTopology?.hosts ?? []);
	let servicesData = $derived(servicesCacheQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Tags query — always up-to-date, survives SSE topology overwrites
	const tagsQuery = useTagsQuery();
	// Sorted alphabetically so the filter view's tag lists are stable + scannable.
	let allTags = $derived([...(tagsQuery.data ?? [])].sort((a, b) => a.name.localeCompare(b.name)));

	// Derive tags that are actually used per entity type
	let hostTagIds = $derived(new Set(hostsData.flatMap((h) => h.tags)));
	let serviceTagIds = $derived(new Set(servicesData.flatMap((s) => s.tags)));
	let subnetTagIds = $derived(new Set(subnetsData.flatMap((s) => s.tags)));

	// Filter tags to only those used by each entity type
	let hostTags = $derived(allTags.filter((t) => hostTagIds.has(t.id)));
	let serviceTags = $derived(allTags.filter((t) => serviceTagIds.has(t.id)));
	let subnetTags = $derived(allTags.filter((t) => subnetTagIds.has(t.id)));

	// Check if there are any untagged entities
	let hasUntaggedHosts = $derived(hostsData.some((h) => h.tags.length === 0));
	let hasUntaggedServices = $derived(servicesData.some((s) => s.tags.length === 0));
	let hasUntaggedSubnets = $derived(subnetsData.some((s) => s.tags.length === 0));

	// Derive filter visibility from element_config
	let viewMetaObj = $derived(
		views.getMetadata($activeView) as {
			element_config?: {
				container_entity: EntityType | null;
				element_entities: Array<{ entity_type: EntityType; inline_entities: EntityType[] }>;
			};
		} | null
	);
	let elementConfig = $derived(viewMetaObj?.element_config);

	// Entity types currently hidden in this view via the eye toggle.
	let hiddenEntitiesThisView = $derived(
		(($topologyOptions.request.hide_entities ?? {}) as Record<string, EntityType[]>)[$activeView] ??
			[]
	);

	interface FilterSection {
		entityType: EntityType;
		roles: { container: boolean; element: boolean; inline: boolean };
		hoverable: boolean;
		togglePresent: boolean;
		toggleDisabled: boolean;
		hidden: boolean;
	}

	// Build the ordered filter-section list from the view config + current
	// hide state. Applies the two-layer toggleability rule — see plan:
	//   static:   togglePresent iff !container-role AND (inline OR (element AND ≥2 elements))
	//   dynamic:  toggleDisabled iff would-hide-last-visible-element
	let filterSections = $derived.by((): FilterSection[] => {
		const config = elementConfig;
		if (!config) return [];
		const elementCount = config.element_entities?.length ?? 0;
		const hiddenElementCount = (config.element_entities ?? []).filter((ee) =>
			hiddenEntitiesThisView.includes(ee.entity_type)
		).length;
		const visibleElementCount = elementCount - hiddenElementCount;

		// Accumulate roles per entity type, preserving order: container first,
		// then each element, then any pure-inline entity we haven't seen yet.
		const order: EntityType[] = [];
		const byEntity = new SvelteMap<EntityType, FilterSection['roles']>();
		const push = (type: EntityType, role: keyof FilterSection['roles']) => {
			let roles = byEntity.get(type);
			if (!roles) {
				roles = { container: false, element: false, inline: false };
				byEntity.set(type, roles);
				order.push(type);
			}
			roles[role] = true;
		};
		if (config.container_entity) push(config.container_entity, 'container');
		for (const ee of config.element_entities ?? []) {
			push(ee.entity_type, 'element');
			for (const inline of ee.inline_entities) push(inline, 'inline');
		}

		return order.map((entityType) => {
			const roles = byEntity.get(entityType)!;
			const hidden = hiddenEntitiesThisView.includes(entityType);
			// Entities with any container role never get an eye — their
			// narrower use cases (e.g. "hide VMs only") are served by
			// metadata filters on that entity instead.
			const togglePresent =
				!roles.container && (roles.inline || (roles.element && elementCount >= 2));
			// Floor: can't hide the last visible element entity.
			const toggleDisabled = togglePresent && !hidden && roles.element && visibleElementCount <= 1;
			// Every rendered section is hoverable. ElementNode's hovered-
			// relationship derived decides the visual treatment per card:
			// element-role = border, inline-role = card glow, otherwise
			// no-op. Pure-inline entities (Port / Service in L3) rely on
			// this path for their highlight affordance.
			return {
				entityType,
				roles,
				hoverable: true,
				togglePresent,
				toggleDisabled,
				hidden
			};
		});
	});

	// Metadata filters declared on the active view, keyed by entity type.
	// Used to render a MetadataFilterGroup per section.
	type MetadataFilterDef = {
		filter_type: string;
		label: string;
		applies: string;
		values: Array<{ id: string; label: string; color: string; icon: string | null }>;
	};
	let metadataFiltersByEntity = $derived(
		(viewMetaObj?.element_config as { metadata_filters?: Record<string, MetadataFilterDef[]> })
			?.metadata_filters ?? {}
	);

	/**
	 * Drop a filter group whose entities all share one value — every toggle
	 * would either show everything or hide everything. Undefined means the
	 * present-value scan hasn't run yet (no topology loaded), in which case the
	 * group is shown rather than flickering out.
	 *
	 * A `Server` filter is always offered, because the response is not evidence of which values
	 * exist: its hidden entities were dropped before the bundle was built. `presentFilterValues`
	 * folds the hidden values back in to compensate, but that only holds while something is
	 * hidden — clear the filter and the fold-back contributes nothing, so the group is judged on
	 * a bundle that has not been refetched yet and disappears, taking with it the only control
	 * that could put the filter back. `hide_metadata_values` reaching the server, the rebuild and
	 * the refetch are three round trips; the panel must not blink out for the length of them.
	 */
	function filterOffersAChoice(entityType: string, filterType: string): boolean {
		const filter = metadataFiltersByEntity[entityType]?.find((f) => f.filter_type === filterType);
		if (filter?.applies === 'Server') return true;
		const present = $presentFilterValues[entityType]?.[filterType];
		return present === undefined || present.length > 1;
	}
	let hiddenMetadataForView = $derived(
		(
			($topologyOptions.request.hide_metadata_values ?? {}) as Record<
				string,
				Record<string, Record<string, string[]>>
			>
		)[$activeView] ?? {}
	);

	function hiddenMetadataValuesFor(entityType: EntityType, filterType: string): string[] {
		return hiddenMetadataForView[entityType]?.[filterType] ?? [];
	}

	// Map from entity type to the tag list / untagged flag for the inline body.
	let tagListByEntity = $derived.by(() => {
		const m: Record<string, { tags: typeof hostTags; hasUntagged: boolean }> = {};
		m['Host'] = { tags: hostTags, hasUntagged: hasUntaggedHosts };
		m['Service'] = { tags: serviceTags, hasUntagged: hasUntaggedServices };
		m['Subnet'] = { tags: subnetTags, hasUntagged: hasUntaggedSubnets };
		return m;
	});

	function onToggleTagForEntity(entityType: EntityType, tagId: string) {
		if (entityType === 'Host') toggleHostTag(tagId);
		else if (entityType === 'Service') toggleServiceTag(tagId);
		else if (entityType === 'Subnet') toggleSubnetTag(tagId);
	}

	function hiddenTagIdsForEntity(entityType: EntityType): string[] {
		const f = $topologyOptions.local.tag_filter;
		if (entityType === 'Host') return f?.hidden_host_tag_ids ?? [];
		if (entityType === 'Service') return f?.hidden_service_tag_ids ?? [];
		if (entityType === 'Subnet') return f?.hidden_subnet_tag_ids ?? [];
		return [];
	}

	function clearedFiltersFor(
		entityType: string,
		existing: Record<string, string[]> | undefined
	): Record<string, string[]> {
		return clearedHideSetFor($activeView, entityType, existing);
	}

	function countHiddenMetadataValues(entityType: EntityType): number {
		const perFilter = hiddenMetadataForView[entityType] ?? {};
		let count = 0;
		for (const filterType of Object.keys(perFilter)) count += perFilter[filterType].length;
		return count;
	}

	/**
	 * Hidden values counted for the badge, defaults included.
	 *
	 * A product default is a filter in force, and the badge is where a user finds out one is —
	 * without it the count reads 0 on a view that is hiding 103 ports, and Clear all (gated on
	 * this) never renders, so there is nothing to press.
	 */
	function userFilterCountFor(entityType: EntityType): number {
		return hiddenTagIdsForEntity(entityType).length + countHiddenMetadataValues(entityType);
	}

	let userFilterTotal = $derived(
		filterSections.reduce((sum, s) => sum + userFilterCountFor(s.entityType), 0)
	);

	function clearFiltersForEntity(entityType: EntityType) {
		const view = $activeView;
		updateTopologyOptions((opts) => {
			const tf = opts.local.tag_filter ?? {
				hidden_host_tag_ids: [],
				hidden_service_tag_ids: [],
				hidden_subnet_tag_ids: []
			};
			const newTf = { ...tf };
			if (entityType === 'Host') newTf.hidden_host_tag_ids = [];
			if (entityType === 'Service') newTf.hidden_service_tag_ids = [];
			if (entityType === 'Subnet') newTf.hidden_subnet_tag_ids = [];

			const hideMeta = {
				...((opts.request.hide_metadata_values ?? {}) as Record<
					string,
					Record<string, Record<string, string[]>>
				>)
			};
			if (hideMeta[view]) {
				const byEntity = { ...hideMeta[view] };
				byEntity[entityType] = clearedFiltersFor(entityType, byEntity[entityType]);
				hideMeta[view] = byEntity;
			}

			return {
				...opts,
				local: { ...opts.local, tag_filter: newTf },
				request: {
					...opts.request,
					hide_metadata_values: hideMeta
				}
			};
		});
	}

	function clearAllFiltersForView() {
		const view = $activeView;
		updateTopologyOptions((opts) => {
			const hideMeta = {
				...((opts.request.hide_metadata_values ?? {}) as Record<
					string,
					Record<string, Record<string, string[]>>
				>)
			};
			const byEntity = { ...(hideMeta[view] ?? {}) };
			for (const entityType of new Set([
				...Object.keys(byEntity),
				...Object.keys(declaredMetadataFiltersFor(view))
			])) {
				byEntity[entityType] = clearedFiltersFor(entityType, byEntity[entityType]);
			}
			hideMeta[view] = byEntity;

			return {
				...opts,
				local: {
					...opts.local,
					tag_filter: {
						hidden_host_tag_ids: [],
						hidden_service_tag_ids: [],
						hidden_subnet_tag_ids: []
					}
				},
				request: {
					...opts.request,
					hide_metadata_values: hideMeta
				}
			};
		});
	}

	function toggleHiddenEntity(entityType: EntityType) {
		clearFiltersForEntity(entityType);
		const view = $activeView;
		updateTopologyOptions((opts) => {
			const map = ((opts.request.hide_entities ?? {}) as Record<string, EntityType[]>) ?? {};
			const current = map[view] ?? [];
			const next = current.includes(entityType)
				? current.filter((e) => e !== entityType)
				: [...current, entityType];
			return {
				...opts,
				request: {
					...opts.request,
					hide_entities: { ...map, [view]: next }
				}
			};
		});
	}

	// Toggle functions for tag filter
	function toggleHostTag(tagId: string) {
		updateTopologyOptions((opts) => {
			const currentFilter = opts.local.tag_filter;
			const hiddenIds = currentFilter?.hidden_host_tag_ids ?? [];
			const idx = hiddenIds.indexOf(tagId);
			const newHiddenIds =
				idx === -1 ? [...hiddenIds, tagId] : hiddenIds.filter((id) => id !== tagId);
			opts.local.tag_filter = {
				hidden_host_tag_ids: newHiddenIds,
				hidden_service_tag_ids: currentFilter?.hidden_service_tag_ids ?? [],
				hidden_subnet_tag_ids: currentFilter?.hidden_subnet_tag_ids ?? []
			};
			return opts;
		});
	}

	function toggleServiceTag(tagId: string) {
		updateTopologyOptions((opts) => {
			const currentFilter = opts.local.tag_filter;
			const hiddenIds = currentFilter?.hidden_service_tag_ids ?? [];
			const idx = hiddenIds.indexOf(tagId);
			const newHiddenIds =
				idx === -1 ? [...hiddenIds, tagId] : hiddenIds.filter((id) => id !== tagId);
			opts.local.tag_filter = {
				hidden_host_tag_ids: currentFilter?.hidden_host_tag_ids ?? [],
				hidden_service_tag_ids: newHiddenIds,
				hidden_subnet_tag_ids: currentFilter?.hidden_subnet_tag_ids ?? []
			};
			return opts;
		});
	}

	function toggleSubnetTag(tagId: string) {
		updateTopologyOptions((opts) => {
			const currentFilter = opts.local.tag_filter;
			const hiddenIds = currentFilter?.hidden_subnet_tag_ids ?? [];
			const idx = hiddenIds.indexOf(tagId);
			const newHiddenIds =
				idx === -1 ? [...hiddenIds, tagId] : hiddenIds.filter((id) => id !== tagId);
			opts.local.tag_filter = {
				hidden_host_tag_ids: currentFilter?.hidden_host_tag_ids ?? [],
				hidden_service_tag_ids: currentFilter?.hidden_service_tag_ids ?? [],
				hidden_subnet_tag_ids: newHiddenIds
			};
			return opts;
		});
	}

	/**
	 * Toggle a single value in the generic metadata-filter hide-set.
	 * Path: request.hide_metadata_values[view][entityType][filterType].
	 */
	function toggleMetadataFilterValue(entityType: EntityType, filterType: string, valueId: string) {
		const view = $activeView;
		updateTopologyOptions((opts) => {
			const map =
				((opts.request.hide_metadata_values ?? {}) as Record<
					string,
					Record<string, Record<string, string[]>>
				>) ?? {};
			const byEntity = { ...(map[view] ?? {}) };
			const byFilter = { ...(byEntity[entityType] ?? {}) };
			const current = byFilter[filterType] ?? [];
			const next = current.includes(valueId)
				? current.filter((v) => v !== valueId)
				: [...current, valueId];
			byFilter[filterType] = next;
			byEntity[entityType] = byFilter;
			return {
				...opts,
				request: {
					...opts.request,
					hide_metadata_values: { ...map, [view]: byEntity }
				}
			};
		});
	}

	function toggleEdgeType(edgeType: string) {
		updateTopologyOptions((opts) => {
			const hidden = opts.local.hide_edge_types ?? [];

			if (edgeType === DEPENDENCIES_GROUP) {
				// Toggle all dependency edge types together
				const allHidden = dependencyEdgeTypeIds.every((id) =>
					hidden.includes(id as (typeof hidden)[number])
				);
				const newHidden = allHidden
					? hidden.filter((e) => !dependencyEdgeTypeIds.includes(e))
					: [
							...hidden.filter((e) => !dependencyEdgeTypeIds.includes(e)),
							...(dependencyEdgeTypeIds as (typeof hidden)[number][])
						];
				return {
					...opts,
					local: { ...opts.local, hide_edge_types: newHidden }
				};
			}

			const idx = hidden.indexOf(edgeType as (typeof hidden)[number]);
			const newHidden =
				idx === -1
					? [...hidden, edgeType as (typeof hidden)[number]]
					: hidden.filter((e) => e !== edgeType);
			return {
				...opts,
				local: {
					...opts.local,
					hide_edge_types: newHidden
				}
			};
		});
	}

	function handleEdgeTypeHoverStart(value: string, color: Color) {
		const types = value === DEPENDENCIES_GROUP ? dependencyEdgeTypeIds : [value];
		hoveredEdgeType.set({ edgeTypes: types, color: color as string });
	}

	function handleEdgeTypeHoverEnd() {
		hoveredEdgeType.set(null);
	}

	let viewMeta = $derived(viewsJson.find((p) => p.id === $activeView));

	// Sentinel value for the unified dependency toggle
	const DEPENDENCIES_GROUP = 'Dependencies';

	// Edges for the active view, read from the enriched topology's built graph
	// (already the active view's flat slice).
	let activeViewEdges = $derived(renderTopology?.edges ?? []);

	// Determine which edge types are dependency edges from metadata
	let dependencyEdgeTypeIds = $derived.by(() => {
		const seen = new SvelteSet<string>();
		const depTypes: string[] = [];
		for (const edge of activeViewEdges) {
			const et = edge.edge_type;
			if (et && !seen.has(et) && !isDisabledEdge(edge)) {
				seen.add(et);
				const meta = edgeTypes.getMetadata(et);
				if (meta?.is_dependency_edge) depTypes.push(et);
			}
		}
		return depTypes;
	});

	// Build edge types with colors from edges present in the topology
	// Dependency edges are collapsed into a single "Dependencies" toggle
	let edgeTypesWithColors = $derived.by(() => {
		const seen: Record<string, boolean> = {};
		const result: { value: string; label: string; color: Color }[] = [];
		let addedDepGroup = false;
		for (const edge of activeViewEdges) {
			const edgeType = edge.edge_type;
			if (edgeType && !seen[edgeType] && !isDisabledEdge(edge)) {
				seen[edgeType] = true;
				const meta = edgeTypes.getMetadata(edgeType);
				if (meta?.is_dependency_edge) {
					if (!addedDepGroup) {
						addedDepGroup = true;
						const colorHelper = edgeTypes.getColorHelper(edgeType);
						result.push({
							value: DEPENDENCIES_GROUP,
							label: common_dependenciesLabel(),
							color: colorHelper.color
						});
					}
				} else {
					const colorHelper = edgeTypes.getColorHelper(edgeType);
					result.push({ value: edgeType, label: edgeType, color: colorHelper.color });
				}
			}
		}
		return result.sort((a, b) => a.label.localeCompare(b.label));
	});

	// Map hide_edge_types to filter-group selected values, replacing individual
	// dependency type IDs with the unified DEPENDENCIES_GROUP sentinel
	let edgeFilterSelectedValues = $derived.by(() => {
		const hidden = $topologyOptions.local.hide_edge_types ?? [];
		const nonDep = hidden.filter((e) => !dependencyEdgeTypeIds.includes(e));
		const allDepHidden =
			dependencyEdgeTypeIds.length > 0 &&
			dependencyEdgeTypeIds.every((id) => hidden.includes(id as (typeof hidden)[number]));
		return allDepHidden ? [...nonDep, DEPENDENCIES_GROUP] : nonDep;
	});

	interface TopologyFieldDef {
		id: string;
		label: () => string;
		type: 'boolean' | 'string';
		path: 'local' | 'request';
		key: string;
		helpText: () => string;
		section: () => string;
		placeholder?: () => string;
	}

	const fieldDefs: TopologyFieldDef[] = [
		// Visual section
		{
			id: 'bundle_edges',
			label: () => topology_bundleEdges(),
			type: 'boolean',
			path: 'local',
			key: 'bundle_edges',
			helpText: () => topology_bundleEdgesHelp(),
			section: () => common_visual()
		},
		{
			id: 'no_fade_edges',
			label: () => topology_dontFadeEdges(),
			type: 'boolean',
			path: 'local',
			key: 'no_fade_edges',
			helpText: () => topology_dontFadeEdgesHelp(),
			section: () => common_visual()
		},
		{
			id: 'show_minimap',
			label: () => topology_showMinimap(),
			type: 'boolean',
			path: 'local',
			key: 'show_minimap',
			helpText: () => topology_showMinimapHelp(),
			section: () => common_visual()
		}
	];

	// Get unique section names in order
	let sectionNames = $derived([...new Set(fieldDefs.map((d) => d.section()))]);

	// Group fields by section
	let sections = $derived(
		sectionNames.map((name) => ({
			name,
			fields: fieldDefs.filter((d) => d.section() === name)
		}))
	);

	// Create form values initialized from topologyOptions
	let values = $state<Record<string, boolean | string | string[]>>({});

	// Initialize values from topologyOptions
	$effect(() => {
		const opts = $topologyOptions;
		const newValues: Record<string, boolean | string | string[]> = {};
		for (const def of fieldDefs) {
			const value =
				def.path === 'local'
					? opts.local[def.key as keyof typeof opts.local]
					: opts.request[def.key as keyof typeof opts.request];
			newValues[def.id] = value as boolean | string | string[];
		}
		values = newValues;
	});

	// Update a field value and sync to topologyOptions
	function updateValue(def: TopologyFieldDef, newValue: boolean | string | string[]) {
		values = { ...values, [def.id]: newValue };

		updateTopologyOptions((opts) => {
			if (def.path === 'local') {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(opts.local as any)[def.key] = newValue;
			} else {
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				(opts.request as any)[def.key] = newValue;
			}
			return opts;
		});
	}
</script>

{#if activeTab === 'filter'}
	<!-- Filters -->
	<div class="space-y-3">
		<div class="flex items-center justify-between gap-2">
			<p class="text-secondary text-xs font-medium">
				{#if userFilterTotal > 0}
					{topology_nFiltersApplied({
						count: userFilterTotal,
						viewName: viewMeta?.name ?? $activeView
					})}
				{:else}
					{topology_filtersApplyToView({ viewName: viewMeta?.name ?? $activeView })}
				{/if}
			</p>
			{#if userFilterTotal > 0}
				<button
					type="button"
					class="btn-secondary shrink-0 gap-1 rounded px-1.5 py-0 text-xs font-medium"
					onclick={clearAllFiltersForView}
				>
					<FunnelX class="h-3 w-3" />
					{common_clearAll()}
				</button>
			{/if}
		</div>

		<div class="space-y-1.5">
			<div class="text-secondary text-xs font-semibold uppercase tracking-wide">
				{common_edges()}
			</div>
			<FilterGroup
				items={edgeTypesWithColors}
				selectedValues={edgeFilterSelectedValues}
				mode="exclude"
				onToggle={toggleEdgeType}
				onHoverStart={handleEdgeTypeHoverStart}
				onHoverEnd={handleEdgeTypeHoverEnd}
			/>
		</div>

		{#each filterSections as section (section.entityType)}
			{@const tagBundle = tagListByEntity[section.entityType]}
			{@const metadataFilters = metadataFiltersByEntity[section.entityType] ?? []}
			{@const hasTagBody = !!tagBundle && (tagBundle.tags.length > 0 || tagBundle.hasUntagged)}
			{@const hasContent = hasTagBody || metadataFilters.length > 0 || section.togglePresent}
			{#if hasContent}
				{@const sectionFilterCount = userFilterCountFor(section.entityType)}
				<div class="filter-section space-y-1.5 border-t border-gray-300 pt-2 dark:border-gray-700">
					<EntityFilterHeader
						entityType={section.entityType}
						hoverable={section.hoverable}
						togglePresent={section.togglePresent}
						toggleDisabled={section.toggleDisabled}
						hidden={section.hidden}
						activeFilterCount={sectionFilterCount}
						onToggle={toggleHiddenEntity}
						onClearSection={clearFiltersForEntity}
					/>
					{#if !section.hidden}
						{#if hasTagBody && tagBundle}
							<TagFilterGroup
								label={metadataFilters.length > 0 ? common_byTag() : undefined}
								tags={tagBundle.tags}
								hiddenTagIds={hiddenTagIdsForEntity(section.entityType)}
								onToggle={(id) => onToggleTagForEntity(section.entityType, id)}
								entityType={section.entityType}
								hasUntagged={tagBundle.hasUntagged}
							/>
						{/if}
						{#each metadataFilters.filter( (f) => filterOffersAChoice(section.entityType, f.filter_type) ) as filter (filter.filter_type)}
							<CategoryFilterGroup
								entityType={section.entityType}
								filterType={filter.filter_type}
								categories={filter.values
									.map((v) => ({
										value: v.id,
										label: v.label,
										color: v.color as Color
									}))
									.sort((a, b) => a.label.localeCompare(b.label))}
								hiddenCategories={hiddenMetadataValuesFor(section.entityType, filter.filter_type)}
								onToggle={(valueId) =>
									toggleMetadataFilterValue(section.entityType, filter.filter_type, valueId)}
								disabled={!editState.isEditable}
								label={filter.label}
							/>
						{/each}
					{/if}
				</div>
			{/if}
		{/each}
	</div>
{:else if activeTab === 'group'}
	<!-- Group By -->
	<div class="space-y-3">
		<p class="text-tertiary text-xs">{topology_groupsHelp()}</p>
		<GroupingRuleEditor />
	</div>
{:else if activeTab === 'visual'}
	<!-- Visual Options -->
	<div class="space-y-3">
		<p class="text-tertiary text-xs">{topology_displayHelp()}</p>
		{#each sections as section (section.name)}
			{#each section.fields as def (def.id)}
				{#if def.type === 'boolean'}
					<OptionToggle
						label={def.label()}
						helpText={def.helpText()}
						path={def.path}
						optionKey={def.key}
						disabled={def.path === 'request' && !editState.isEditable}
						disabledReason={def.path === 'request' && !editState.isEditable
							? getOptionDisabledTooltip()
							: ''}
					/>
				{:else if def.type === 'string'}
					<div>
						<label for={def.id} class="text-secondary mb-1 block text-sm font-medium">
							{def.label()}
						</label>
						<input
							type="text"
							id={def.id}
							class="input-field w-full"
							placeholder={def.placeholder?.() ?? ''}
							value={values[def.id] ?? ''}
							oninput={(e) => updateValue(def, e.currentTarget.value)}
						/>
						{#if def.helpText}
							<p class="text-tertiary mt-1 text-xs">{def.helpText()}</p>
						{/if}
					</div>
				{/if}
			{/each}
		{/each}
	</div>
{/if}
