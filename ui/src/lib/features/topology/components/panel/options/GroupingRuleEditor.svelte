<script lang="ts">
	import { Edit, Check, Eye, EyeOff } from '@lucide/svelte';
	import ListManager from '#lib/shared/components/forms/selection/ListManager.svelte';
	import { SimpleOptionDisplay } from '#lib/shared/components/forms/selection/display/SimpleOptionDisplay.ts';
	import FilterGroup from './FilterGroup.svelte';
	import GroupingRuleItem from './GroupingRuleItem.svelte';
	import type {
		ContainerGraphRule,
		ElementGraphRule,
		ElementRule,
		ContainerRule
	} from '../../../types/grouping';
	import {
		setElementRuleTitle,
		getElementRuleType,
		getElementRuleTitle,
		makeGraphRule,
		getContainerRuleDiscriminant
	} from '../../../types/grouping';
	import {
		topologyOptions,
		updateTopologyOptions,
		activeView,
		topologyReadOnly,
		sharedElementRules,
		updateSharedElementRules,
		getInfrastructureRuleId,
		getOrgUseCase
	} from '../../../queries';
	import { getUseCases } from '#lib/features/auth/types/base.ts';
	import { getTopologyEditState } from '../../../state';
	import {
		useTopologiesQuery,
		useUpdateTopologyMutation,
		selectedTopologyId,
		sanitizeOptionsForApi
	} from '../../../queries';
	import type { components } from '#lib/api/schema';
	type ServiceCategory = components['schemas']['ServiceCategory'];
	type Entity = components['schemas']['EntityDiscriminants'];
	import { useTagsQuery } from '#lib/features/tags/queries.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { formatEntityLabel, formatEntityLabelTitle } from '#lib/features/topology/labels.ts';
	import { SvelteSet } from 'svelte/reactivity';
	import { type Color } from '#lib/shared/utils/styling.ts';
	interface RuleTypeMetadata<R> {
		id: string;
		name: string | null;
		description: string | null;
		category: string | null;
		icon: string | null;
		color: string | null;
		metadata: {
			is_removable: boolean;
			is_reorderable: boolean;
			is_configurable: boolean;
			allow_multiple: boolean;
			views?: string[];
			/** The rule as "Add rule" creates it. */
			default_payload: R;
		} | null;
	}
	import _containerRuleTypes from '#lib/data/container-rule-types.json';
	import _elementRuleTypes from '#lib/data/element-rule-types.json';
	const typedContainerRuleTypes = _containerRuleTypes as RuleTypeMetadata<ContainerRule>[];
	const typedElementRuleTypes = _elementRuleTypes as RuleTypeMetadata<ElementRule>[];
	import {
		topology_containerGroupingHelp,
		topology_containerGroupingPerspective,
		topology_elementGrouping,
		topology_addContainerRule,
		topology_addElementRule,
		topology_elementGroupingHelp,
		topology_elementRuleNotApplicable,
		topology_groupRuleTitlePlaceholder,
		topology_showDisabledRules,
		topology_hideDisabledRules,
		topology_ruleBuiltInRemove,
		topology_ruleBuiltInReorder,
		topology_ruleNoSettings,
		topology_showAllToReorder,
		topology_noHiddenRules,
		topology_byTagRuleDescription,
		topology_infraRuleDescription,
		topology_infraRuleNote,
		topology_groupingReadOnlySnapshot
	} from '#lib/paraglide/messages.js';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import viewsJson from '#lib/data/views.json';
	import { metaName, metaDescription } from '#lib/i18n/metadata.ts';
	import { disabledWhen, hiddenWhen, type Unavailable } from '#lib/shared/components/data/types.ts';

	// Topology for edit state and option-saving
	const topologiesQuery = useTopologiesQuery();
	const updateTopologyMutation = useUpdateTopologyMutation();
	let topologiesData = $derived(topologiesQuery.data ?? []);
	let topology = $derived(topologiesData.find((t) => t.id === $selectedTopologyId));
	let editState = $derived(getTopologyEditState(topology, false, $topologyReadOnly));

	function saveOptions() {
		if (!topology) return;
		updateTopologyMutation.mutate({
			...topology,
			options: sanitizeOptionsForApi(topology.options)
		});
	}

	// Tags query
	const tagsQuery = useTagsQuery();
	let allTags = $derived(tagsQuery.data ?? []);

	// Use case label for infra rule note
	let useCaseLabel = $derived.by(() => {
		const useCase = getOrgUseCase();
		return getUseCases()[useCase as keyof ReturnType<typeof getUseCases>]?.label ?? useCase;
	});

	// Container rules for the active view (per-view HashMap)
	let containerRules = $derived(
		((
			$topologyOptions.request.container_rules as Record<string, ContainerGraphRule[]> | undefined
		)?.[$activeView] ?? []) as ContainerGraphRule[]
	);

	// Element rules from shared store (committed state, cross-view)
	let committedElementRules = $derived($sharedElementRules);

	// Pending edits buffer: used while an editor is open so individual toggles
	// don't trigger rebuilds. Flushed to the store on checkmark close.
	let pendingElementRules = $state<ElementGraphRule[] | null>(null);

	// Active rules: pending edits if editing, otherwise committed
	let elementRules = $derived(pendingElementRules ?? committedElementRules);

	// Editing state tracked by rule UUID + the view it was opened on.
	// When the view changes, activeEditingId becomes null so the editor
	// collapses. The next edit toggle flushes stale pending state.
	let editingElementId = $state<string | null>(null);
	let editingView = $state<string | null>(null);

	// Show/hide disabled (non-applicable) element rules
	let hideDisabledElementRules = $state(false);

	// Metadata lookups
	const containerRuleMeta = Object.fromEntries(typedContainerRuleTypes.map((m) => [m.id, m]));
	const elementRuleMeta = Object.fromEntries(typedElementRuleTypes.map((m) => [m.id, m]));

	// Perspective name lookup for tooltip
	const viewNames = Object.fromEntries(viewsJson.map((p) => [p.id, p.name ?? p.id]));

	// Translated names/descriptions (meta_* i18n keys with fixture fallback)
	function viewName(id: string): string {
		return metaName('views', id, viewNames[id] ?? id);
	}
	function containerRuleName(id: string): string {
		return metaName('container_rule_types', id, containerRuleMeta[id]?.name ?? id);
	}
	function containerRuleDescription(id: string): string | undefined {
		const raw = containerRuleMeta[id]?.description;
		return raw ? metaDescription('container_rule_types', id, raw) : undefined;
	}
	function elementRuleName(id: string): string {
		return metaName('element_rule_types', id, elementRuleMeta[id]?.name ?? id);
	}
	function elementRuleDescription(id: string): string | undefined {
		const raw = elementRuleMeta[id]?.description;
		return raw ? metaDescription('element_rule_types', id, raw) : undefined;
	}

	/** Whether an element rule applies to the current view */
	function isElementRuleApplicable(item: ElementGraphRule): boolean {
		const ruleId = getElementRuleType(item.rule);
		const meta = elementRuleMeta[ruleId];
		const applicableViews = meta?.metadata?.views;
		return !applicableViews || applicableViews.includes(currentView);
	}

	/** Tooltip for a disabled (non-applicable) element rule */
	function getElementRuleDisabledTooltip(item: ElementGraphRule): string | undefined {
		if (isElementRuleApplicable(item)) return undefined;
		const ruleId = getElementRuleType(item.rule);
		const meta = elementRuleMeta[ruleId];
		const applicableViews = meta?.metadata?.views ?? [];
		const names = applicableViews.map((p: string) => viewName(p));
		return topology_elementRuleNotApplicable({ perspectives: names.join(', ') });
	}

	// Filtered element rules and index mapping for show/hide toggle
	let visibleElementRules = $derived(
		hideDisabledElementRules ? elementRules.filter((r) => isElementRuleApplicable(r)) : elementRules
	);
	let visibleToRealIndexMap = $derived(
		hideDisabledElementRules
			? elementRules.reduce<number[]>((map, r, i) => {
					if (isElementRuleApplicable(r)) map.push(i);
					return map;
				}, [])
			: elementRules.map((_, i) => i)
	);
	let disabledCount = $derived(elementRules.filter((r) => !isElementRuleApplicable(r)).length);

	// Service categories from fixture data (all categories, not just those in current topology)
	import serviceCategoriesJson from '#lib/data/service-categories.json';
	let serviceCategoriesWithColors = serviceCategoriesJson
		.map((cat) => ({
			value: cat.id,
			label: cat.name,
			color: (cat.color ?? 'Gray') as Color,
			tooltip: cat.description || undefined
		}))
		.sort((a, b) => a.label.localeCompare(b.label));

	// All tags as toggleable pills, sorted alphabetically (matches the
	// service-category pills, which are already sorted).
	let allTagsWithColors = $derived(
		allTags
			.map((t) => ({
				value: t.id,
				label: t.name,
				color: t.color as Color
			}))
			.sort((a, b) => a.label.localeCompare(b.label))
	);

	// --- Container Rules ---

	let containerAddOptions = $derived.by(() => {
		return filteredContainerRuleTypes
			.filter(
				(m) =>
					m.metadata?.is_removable &&
					!containerRules.some((r) => getContainerRuleDiscriminant(r.rule) === m.id)
			)
			.map((m) => ({
				value: m.id,
				label: containerRuleName(m.id),
				description: containerRuleDescription(m.id)
			}));
	});

	const containerRuleDisplayComponent = {
		getId: (item: ContainerGraphRule) => item.id,
		getLabel: (item: ContainerGraphRule) =>
			containerRuleName(getContainerRuleDiscriminant(item.rule))
	};

	let currentView = $derived($activeView);
	let activeEditingId = $derived(editingView === currentView ? editingElementId : null);

	let viewMeta = $derived(viewsJson.find((p) => p.id === currentView));

	/** The element entities this view actually renders, top-level only. */
	let viewElementEntities = $derived.by((): Entity[] => {
		const elementEntities =
			(
				(viewMeta?.metadata as Record<string, unknown>)?.element_config as
					| { element_entities?: Array<{ entity_type: Entity; inline_entities: Entity[] }> }
					| undefined
			)?.element_entities ?? [];
		// Top-level entities only: an inline entity is drawn *inside* another element's card
		// (a Service on an IP address), so it is not what the view is a view of.
		return [...new SvelteSet(elementEntities.map((config) => config.entity_type))];
	});

	// Resolve each element entity to its nearest taggable ancestor (or itself
	// if taggable), deduped — drives the ByTag rule description, which has to name
	// what you can actually tag rather than what is on screen.
	let viewTaggableEntities = $derived.by((): Entity[] => {
		const elementEntities =
			(
				(viewMeta?.metadata as Record<string, unknown>)?.element_config as
					| { element_entities?: Array<{ entity_type: Entity; inline_entities: Entity[] }> }
					| undefined
			)?.element_entities ?? [];
		const resolved = new SvelteSet<Entity>();
		// Each config entry is { entity_type, inline_entities }; both the top-level
		// entity and its inline entities render as elements the ByTag rule can match,
		// so resolve every one to its nearest taggable ancestor (or itself).
		for (const config of elementEntities) {
			for (const e of [config.entity_type, ...(config.inline_entities ?? [])]) {
				const meta = entities.getMetadata(e);
				if (meta?.is_taggable) {
					resolved.add(e);
				} else if (meta?.parent_taggable_entity) {
					resolved.add(meta.parent_taggable_entity as Entity);
				}
			}
		}
		return [...resolved];
	});

	// Named for what the view renders, not for what those elements can be tagged by: L2 groups
	// Interfaces and L3 groups IP Addresses, and both resolve to Host for tagging purposes.
	// Sharing one value made every view whose elements are not themselves taggable name the
	// wrong thing, while Workloads and Application looked correct only because theirs are.
	let elementGroupingLabel = $derived(formatEntityLabelTitle(viewElementEntities));
	let elementGroupingLabelPlural = $derived(formatEntityLabel(viewElementEntities));

	let byTagRuleDescription = $derived(
		topology_byTagRuleDescription({
			view: viewName(currentView),
			entities: formatEntityLabel(viewTaggableEntities)
		})
	);

	let filteredContainerRuleTypes = $derived(
		typedContainerRuleTypes.filter((m) => {
			const applicableViews = m.metadata?.views;
			return !applicableViews || applicableViews.includes(currentView);
		})
	);

	let filteredElementRuleTypes = $derived(
		typedElementRuleTypes.filter((m) => m.metadata?.is_removable)
	);

	function updateContainerRules(newRules: ContainerGraphRule[]) {
		updateTopologyOptions((opts) => ({
			...opts,
			request: {
				...opts.request,
				container_rules: {
					...(opts.request.container_rules as Record<string, ContainerGraphRule[]>),
					[$activeView]: newRules
				}
			}
		}));
	}

	function handleContainerAdd(optionId: string) {
		const rule = containerRuleMeta[optionId]?.metadata?.default_payload;
		if (!rule) return;
		updateContainerRules([...containerRules, makeGraphRule(structuredClone(rule))]);
	}

	function handleContainerRemove(index: number) {
		updateContainerRules(containerRules.filter((_, i) => i !== index));
	}

	function handleContainerMoveUp(fromIndex: number) {
		if (fromIndex <= 0) return;
		const newRules = [...containerRules];
		[newRules[fromIndex - 1], newRules[fromIndex]] = [newRules[fromIndex], newRules[fromIndex - 1]];
		updateContainerRules(newRules);
	}

	function handleContainerMoveDown(fromIndex: number) {
		if (fromIndex >= containerRules.length - 1) return;
		const newRules = [...containerRules];
		[newRules[fromIndex], newRules[fromIndex + 1]] = [newRules[fromIndex + 1], newRules[fromIndex]];
		updateContainerRules(newRules);
	}

	// All edit/add/remove/reorder affordances follow read-only: in a snapshot (or
	// any read-only topology) the grouping rules are view-only.
	function canRemoveContainerRule(rule: ContainerGraphRule): boolean {
		if (!editState.isEditable) return false;
		const meta = containerRuleMeta[getContainerRuleDiscriminant(rule.rule)];
		return meta?.metadata?.is_removable ?? false;
	}

	function canReorderContainerRule(rule: ContainerGraphRule): boolean {
		if (!editState.isEditable) return false;
		const meta = containerRuleMeta[getContainerRuleDiscriminant(rule.rule)];
		return meta?.metadata?.is_reorderable ?? false;
	}

	function canRemoveElementRule(item: ElementGraphRule): boolean {
		if (!editState.isEditable) return false;
		const meta = elementRuleMeta[getElementRuleType(item.rule)];
		if (!meta?.metadata?.is_removable) return false;
		if (item.id === getInfrastructureRuleId()) return false;
		return true;
	}

	function canReorderElementRule(item: ElementGraphRule): boolean {
		if (!editState.isEditable) return false;
		const meta = elementRuleMeta[getElementRuleType(item.rule)];
		if (!meta?.metadata?.is_reorderable) return false;
		if (item.id === getInfrastructureRuleId()) return false;
		return true;
	}

	function canConfigureElementRule(item: ElementGraphRule): boolean {
		if (!editState.isEditable) return false;
		const meta = elementRuleMeta[getElementRuleType(item.rule)];
		return meta?.metadata?.is_configurable ?? false;
	}

	function isElementRuleLocked(item: ElementGraphRule): boolean {
		return !canRemoveElementRule(item) && !canReorderElementRule(item);
	}

	// Settings exist only on configurable rule types, so the others never show an edit action. A
	// rule that does nothing in this view shows edit and remove disabled, with the views it
	// applies to.
	function elementRuleEditUnavailable(item: ElementGraphRule): Unavailable | undefined {
		const notApplicable = getElementRuleDisabledTooltip(item);
		return (
			hiddenWhen(!canConfigureElementRule(item), topology_ruleNoSettings()) ??
			disabledWhen(!!notApplicable, notApplicable ?? '')
		);
	}

	function elementRuleRemoveUnavailable(item: ElementGraphRule): Unavailable | undefined {
		const notApplicable = getElementRuleDisabledTooltip(item);
		return (
			disabledWhen(!canRemoveElementRule(item), topology_ruleBuiltInRemove()) ??
			disabledWhen(!!notApplicable, notApplicable ?? '')
		);
	}

	// --- Element Rules ---

	let elementAddOptions = $derived(
		filteredElementRuleTypes.map((m) => {
			const applicableViews = m.metadata?.views;
			const isApplicable = !applicableViews || applicableViews.includes(currentView);
			const alreadyPresent =
				!m.metadata?.allow_multiple &&
				elementRules.some((r) => getElementRuleType(r.rule) === m.id);

			let disabled = false;
			let disabledReason: string | undefined;

			if (!isApplicable) {
				disabled = true;
				const viewNames = (applicableViews ?? []).join(', ');
				disabledReason = topology_elementRuleNotApplicable({ perspectives: viewNames });
			} else if (alreadyPresent) {
				disabled = true;
			}

			return {
				value: m.id,
				label: elementRuleName(m.id),
				description: elementRuleDescription(m.id),
				disabled,
				disabledReason
			};
		})
	);

	const elementRuleDisplayComponent = {
		getId: (item: ElementGraphRule) => item.id,
		getLabel: (item: ElementGraphRule) => getElementRuleType(item.rule)
	};

	function getElementRuleLabel(item: ElementGraphRule): string {
		const ruleType = getElementRuleType(item.rule);
		const title = getElementRuleTitle(item.rule);
		const typeName = elementRuleMeta[ruleType]?.name ? elementRuleName(ruleType) : '';
		return title ?? typeName;
	}

	function updateElementRules(newRules: ElementGraphRule[]) {
		updateSharedElementRules(() => newRules);
	}

	function handleElementAdd(optionId: string) {
		const meta = elementRuleMeta[optionId]?.metadata;
		if (!meta) return;
		const graphRule = makeGraphRule(structuredClone(meta.default_payload));
		updateElementRules([...elementRules, graphRule]);
		if (meta.is_configurable) editingElementId = graphRule.id;
	}

	function handleElementRemove(index: number) {
		// Always operate on committed state to avoid flushing pending edits
		const rules = committedElementRules;
		const removedId = rules[index]?.id;
		updateElementRules(rules.filter((_, i) => i !== index));
		// Keep pending buffer in sync if active
		if (pendingElementRules) {
			pendingElementRules = pendingElementRules.filter((r) => r.id !== removedId);
		}
		if (editingElementId === removedId) editingElementId = null;
		saveOptions();
	}

	function handleElementMoveUp(fromIndex: number) {
		if (fromIndex <= 0) return;
		const newRules = [...elementRules];
		[newRules[fromIndex - 1], newRules[fromIndex]] = [newRules[fromIndex], newRules[fromIndex - 1]];
		updateElementRules(newRules);
		saveOptions();
	}

	function handleElementMoveDown(fromIndex: number) {
		if (fromIndex >= elementRules.length - 1) return;
		const newRules = [...elementRules];
		[newRules[fromIndex], newRules[fromIndex + 1]] = [newRules[fromIndex + 1], newRules[fromIndex]];
		updateElementRules(newRules);
		saveOptions();
	}

	function handleElementEdit(_item: ElementGraphRule, index: number) {
		// If there's a stale editor from a different view, flush and reset first
		if (editingElementId && editingView !== currentView) {
			if (pendingElementRules) {
				updateElementRules(pendingElementRules);
				pendingElementRules = null;
			}
			editingElementId = null;
			editingView = null;
		}

		const ruleId = elementRules[index]?.id;
		const wasEditing = editingElementId === ruleId;
		editingElementId = wasEditing ? null : ruleId;
		editingView = wasEditing ? null : currentView;

		if (wasEditing) {
			// Closing editor: flush pending edits to store; the topology
			// options store subscriber auto-saves to the backend.
			if (pendingElementRules) {
				updateElementRules(pendingElementRules);
				pendingElementRules = null;
			}
			saveOptions();
		} else {
			// Opening editor: start buffering edits
			pendingElementRules = [...elementRules];
		}
	}

	function isElementEditing(item: ElementGraphRule): boolean {
		return item.id === activeEditingId;
	}

	function getElementEditIcon(item: ElementGraphRule) {
		return isElementEditing(item) ? Check : Edit;
	}

	function getElementEditButtonClass(item: ElementGraphRule): string {
		return isElementEditing(item) ? 'btn-icon-success' : 'btn-icon';
	}

	function isElementItemEditing(item: ElementGraphRule): boolean {
		return isElementEditing(item);
	}

	// Wrappers that map visible indices to real indices when filtering
	function handleVisibleElementRemove(visibleIndex: number) {
		handleElementRemove(visibleToRealIndexMap[visibleIndex]);
	}
	function handleVisibleElementEdit(item: ElementGraphRule, visibleIndex: number) {
		handleElementEdit(item, visibleToRealIndexMap[visibleIndex]);
	}

	function bufferElementEdit(updater: (rules: ElementGraphRule[]) => ElementGraphRule[]) {
		pendingElementRules = updater(elementRules);
	}

	function handleElementTitleChange(index: number, title: string | null) {
		bufferElementEdit((rules) => {
			const newRules = [...rules];
			newRules[index] = {
				...newRules[index],
				rule: setElementRuleTitle(newRules[index].rule, title)
			};
			return newRules;
		});
	}

	function handleTagToggle(index: number, tagId: string) {
		const item = elementRules[index];
		if (typeof item.rule === 'object' && 'ByTag' in item.rule) {
			const byTag = item.rule.ByTag;
			const current = byTag.tag_ids ?? [];
			const idx = current.indexOf(tagId);
			const newTagIds = idx === -1 ? [...current, tagId] : current.filter((id) => id !== tagId);
			bufferElementEdit((rules) => {
				const newRules = [...rules];
				newRules[index] = {
					...item,
					rule: { ByTag: { ...byTag, tag_ids: newTagIds } }
				};
				return newRules;
			});
		}
	}

	function toggleCategory(index: number, category: ServiceCategory) {
		const item = elementRules[index];
		if (typeof item.rule === 'object' && 'ByServiceCategory' in item.rule) {
			const byCat = item.rule.ByServiceCategory;
			const current = byCat.categories;
			const idx = current.indexOf(category);
			const newCategories: ServiceCategory[] =
				idx === -1 ? [...current, category] : current.filter((c) => c !== category);
			bufferElementEdit((rules) => {
				const newRules = [...rules];
				newRules[index] = {
					...item,
					rule: {
						ByServiceCategory: { ...byCat, categories: newCategories }
					}
				};
				return newRules;
			});
		}
	}
</script>

{#if !editState.isEditable}
	<div class="mb-3">
		<InlineInfo title="" body={topology_groupingReadOnlySnapshot()} />
	</div>
{/if}

<!-- Container grouping section -->
<div class="mb-4">
	<ListManager
		label={topology_containerGroupingPerspective({
			perspective: viewMeta ? viewName(currentView) : ''
		})}
		placeholder={topology_addContainerRule()}
		items={containerRules}
		options={containerAddOptions}
		optionDisplayComponent={SimpleOptionDisplay}
		itemDisplayComponent={containerRuleDisplayComponent}
		allowReorder={editState.isEditable}
		allowDuplicates={false}
		allowAddFromOptions={editState.isEditable && containerAddOptions.length > 0}
		allowItemEdit={false}
		allowItemRemove={editState.isEditable}
		itemRemoveUnavailable={(item) =>
			disabledWhen(!canRemoveContainerRule(item), topology_ruleBuiltInRemove())}
		itemReorderUnavailable={(item) =>
			disabledWhen(!canReorderContainerRule(item), topology_ruleBuiltInReorder())}
		onAdd={handleContainerAdd}
		onRemove={handleContainerRemove}
		onMoveUp={handleContainerMoveUp}
		onMoveDown={handleContainerMoveDown}
	>
		{#snippet helpSnippet()}
			<p class="text-xs text-tertiary">{topology_containerGroupingHelp()}</p>
		{/snippet}
		{#snippet itemSnippet({ item })}
			<GroupingRuleItem
				label={containerRuleName(getContainerRuleDiscriminant(item.rule))}
				description={containerRuleDescription(getContainerRuleDiscriminant(item.rule))}
				locked={!canReorderContainerRule(item) && !canRemoveContainerRule(item)}
			/>
		{/snippet}
	</ListManager>
</div>

<!-- Element grouping section -->
<ListManager
	label={topology_elementGrouping({ label: elementGroupingLabel })}
	placeholder={topology_addElementRule()}
	items={visibleElementRules}
	options={elementAddOptions}
	optionDisplayComponent={SimpleOptionDisplay}
	itemDisplayComponent={elementRuleDisplayComponent}
	allowReorder={editState.isEditable}
	allowDuplicates={true}
	allowAddFromOptions={editState.isEditable && elementAddOptions.length > 0}
	allowItemEdit={editState.isEditable}
	allowItemRemove={editState.isEditable}
	itemEditUnavailable={elementRuleEditUnavailable}
	itemRemoveUnavailable={elementRuleRemoveUnavailable}
	itemReorderUnavailable={(item) =>
		disabledWhen(!canReorderElementRule(item), topology_ruleBuiltInReorder())}
	reorderDisabledTooltip={hideDisabledElementRules ? topology_showAllToReorder() : undefined}
	editIcon={getElementEditIcon}
	editButtonClass={getElementEditButtonClass}
	isItemEditing={isElementItemEditing}
	onAdd={handleElementAdd}
	onRemove={handleVisibleElementRemove}
	onMoveUp={handleElementMoveUp}
	onMoveDown={handleElementMoveDown}
	onEdit={handleVisibleElementEdit}
>
	{#snippet helpSnippet()}
		<p class="text-xs text-tertiary">
			{topology_elementGroupingHelp({ label: elementGroupingLabelPlural })}
		</p>
	{/snippet}
	{#snippet headerSnippet()}
		<button
			type="button"
			class="btn-icon flex items-center gap-1 text-xs"
			class:opacity-50={disabledCount === 0}
			disabled={disabledCount === 0}
			title={disabledCount === 0
				? topology_noHiddenRules()
				: hideDisabledElementRules
					? topology_showDisabledRules()
					: topology_hideDisabledRules({ count: String(disabledCount) })}
			onclick={() => (hideDisabledElementRules = !hideDisabledElementRules)}
		>
			{#if hideDisabledElementRules}
				<EyeOff size={14} />
			{:else}
				<Eye size={14} />
			{/if}
			{#if disabledCount > 0}
				<span class="text-tertiary">({disabledCount})</span>
			{/if}
		</button>
	{/snippet}
	{#snippet itemSnippet({ item })}
		<GroupingRuleItem
			label={getElementRuleLabel(item)}
			description={item.id === getInfrastructureRuleId()
				? topology_infraRuleDescription()
				: getElementRuleType(item.rule) === 'ByTag'
					? byTagRuleDescription
					: elementRuleDescription(getElementRuleType(item.rule))}
			disabled={!isElementRuleApplicable(item)}
			locked={isElementRuleLocked(item)}
			disabledTooltip={getElementRuleDisabledTooltip(item)}
		/>
	{/snippet}
	{#snippet itemExpandedSnippet({ item, index })}
		{@const realIndex = visibleToRealIndexMap[index]}
		{#if item.id === activeEditingId && typeof item.rule !== 'string'}
			{@const rule = item.rule}
			<!-- Click/keydown stopPropagation keeps edits local to this
			     expanded row (prevents parent list from re-selecting). -->
			<div
				role="presentation"
				onclick={(e) => e.stopPropagation()}
				onkeydown={(e) => e.stopPropagation()}
				class="mt-2 w-full space-y-3 border-t border-gray-200 pt-2 dark:border-gray-700"
			>
				{#if item.id === getInfrastructureRuleId()}
					<p class="text-xs text-tertiary">{topology_infraRuleNote({ useCase: useCaseLabel })}</p>
				{/if}
				<!-- Title input (hidden for infrastructure rule — title is system-managed) -->
				{#if item.id !== getInfrastructureRuleId()}
					<input
						type="text"
						class="input-field w-full py-1 text-sm"
						placeholder={topology_groupRuleTitlePlaceholder()}
						value={getElementRuleTitle(item.rule) ?? ''}
						oninput={(e) =>
							handleElementTitleChange(
								realIndex,
								(e.currentTarget as HTMLInputElement).value || null
							)}
						disabled={!editState.isEditable}
					/>
				{/if}

				<!-- Selection pills -->
				{#if 'ByServiceCategory' in rule}
					<FilterGroup
						items={serviceCategoriesWithColors}
						selectedValues={rule.ByServiceCategory.categories}
						mode="include"
						onToggle={(cat) => toggleCategory(realIndex, cat as ServiceCategory)}
						disabled={!editState.isEditable}
						nativeTooltip={true}
					/>
				{:else if 'ByTag' in rule}
					<FilterGroup
						items={allTagsWithColors}
						selectedValues={rule.ByTag.tag_ids}
						mode="include"
						onToggle={(tagId) => handleTagToggle(realIndex, tagId)}
						disabled={!editState.isEditable}
					/>
				{/if}
			</div>
		{/if}
	{/snippet}
</ListManager>
