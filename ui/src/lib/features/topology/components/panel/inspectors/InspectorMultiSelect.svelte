<script lang="ts">
	import { get } from 'svelte/store';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { Eye, EyeOff, X, Crosshair, ArrowDown } from 'lucide-svelte';
	import { useSvelteFlow } from '@xyflow/svelte';
	import {
		selectedNodes,
		previewEdges,
		baseFlowEdges,
		activeView,
		topologyOptions,
		topologyReadOnly,
		updateSharedElementRules
	} from '../../../queries';
	import type { RenderableTopology } from '../../../types/base';
	import type { TopologyNode } from '../../../types/base';
	import {
		getNodeSelectionIds,
		resolveDependencyTargets,
		resolveEditDependencyTargets,
		resolveTagTarget,
		type DependencyTarget
	} from '../../../resolvers';
	import type { DependencyType, EdgeStyle } from '$lib/features/dependencies/types/base';
	import { generateDependencyName } from '$lib/features/dependencies/utils';
	import { getTopologyEditState } from '../../../state';
	import { computeCommonTags } from '$lib/shared/utils/tags';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import {
		useTagsQuery,
		useBulkAddTagMutation,
		useBulkRemoveTagMutation
	} from '$lib/features/tags/queries';
	import {
		useCreateDependencyMutation,
		useUpdateDependencyMutation,
		createEmptyDependencyFormData
	} from '$lib/features/dependencies/queries';
	import type { Dependency } from '$lib/features/dependencies/types/base';
	import EdgeStyleForm from '$lib/features/dependencies/components/DependencyEditModal/EdgeStyleForm.svelte';
	import { computeOptimalHandles } from '../../../layout/elk-layout';
	import { dependencyTypes, concepts } from '$lib/shared/stores/metadata';
	import {
		commonTagsHeader,
		formatEntityCounts,
		noCommonTagsHintText,
		tallySelection
	} from '$lib/features/topology/labels';
	import { getInspectorConfig } from './view-config';
	import DependencyTargetCard from './shared/DependencyTargetCard.svelte';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import { required, max } from '$lib/shared/components/forms/validators';
	import type { Node, Edge } from '@xyflow/svelte';
	import type { Color } from '$lib/shared/utils/styling';
	import { AVAILABLE_COLORS, createColorHelper } from '$lib/shared/utils/styling';
	import { browser } from '$app/environment';
	import { v4 as uuidv4 } from 'uuid';
	import {
		appWizard_selectedCount,
		topology_multiSelectReadOnlyHint,
		common_clearSelection,
		common_dependency,
		common_entityName,
		common_hub,
		common_spokes,
		common_makesRequestTo,
		common_serves,
		topology_hidePreview,
		topology_showPreview,
		common_cancel,
		common_update,
		dependencies_createDependency,
		dependencies_editDependency,
		dependencies_servicesOnly,
		dependencies_servicesOnlyL3Hint,
		dependencies_withPorts,
		dependencies_withPortsL3Hint,
		topology_multiSelectPreviewEdge,
		topology_focusSelection,
		tags_crossGroupSelectionHint,
		tags_inheritedFromHost,
		tags_inheritedOverrideHint,
		inspector_createGroupingRuleFromTag,
		topology_tutorialDone,
		common_application,
		common_ungrouped
	} from '$lib/paraglide/messages';

	let {
		topology,
		isReadOnly = false,
		isTutorial = false,
		onClearSelection,
		onGroupCreated,
		onDependencyTypeChange,
		editingDependency = null,
		onDone
	}: {
		topology: RenderableTopology | undefined;
		isReadOnly?: boolean;
		isTutorial?: boolean;
		onClearSelection: () => void;
		onGroupCreated?: (groupId: string) => void;
		onDependencyTypeChange?: (type: DependencyType) => void;
		/** When set, the component acts as an edit form for an existing dependency
		 *  (PUT on submit) instead of a create form. $selectedNodes is ignored. */
		editingDependency?: Dependency | null;
		/** Fired when the user finishes editing (successful Update) or cancels. */
		onDone?: () => void;
	} = $props();

	let isEditMode = $derived(editingDependency !== null);

	const { fitView, getInternalNode } = useSvelteFlow();
	const PREVIEW_STORAGE_KEY = 'scanopy_topology_group_preview';

	const bulkAddTagMutation = useBulkAddTagMutation();
	const bulkRemoveTagMutation = useBulkRemoveTagMutation();
	const createDependencyMutation = useCreateDependencyMutation();
	const updateDependencyMutation = useUpdateDependencyMutation();

	// Subscribe to selectedNodes
	let nodes = $state<Node[]>(get(selectedNodes));
	selectedNodes.subscribe((value) => {
		nodes = value;
	});

	// Bulk-tag selection uses the legacy fan-out resolver (tagging a host = tagging its services)
	let selectionIds = $derived.by(() => {
		if (!topology) return { hostIds: [] as string[], serviceIds: [] as string[] };
		const hostSet = new SvelteSet<string>();
		const serviceSet = new SvelteSet<string>();
		for (const node of nodes) {
			const ids = getNodeSelectionIds(node.id, node.data as TopologyNode, topology);
			ids.hostIds.forEach((id) => hostSet.add(id));
			ids.serviceIds.forEach((id) => serviceSet.add(id));
		}
		return { hostIds: [...hostSet], serviceIds: [...serviceSet] };
	});

	let selectedServiceIds = $derived(selectionIds.serviceIds);
	let selectedServices = $derived(
		topology ? topology.services.filter((s) => selectedServiceIds.includes(s.id)) : []
	);

	// View-driven config
	let inspectorConfig = $derived(getInspectorConfig($activeView));
	let selectionSummary = $derived(formatEntityCounts(tallySelection(nodes)));

	// Group selected nodes by their resolved taggable entity type. Each group drives
	// its own tag picker. IPAddress/Interface elements resolve to Host via parent_taggable_entity;
	// Service/Host elements resolve to themselves.
	interface TagGroup {
		entityType: 'Host' | 'Service';
		entityIds: string[];
		entities: Array<{ id: string; tags: string[] }>;
		commonTags: string[];
		label: string;
	}

	let tagGroups = $derived.by((): TagGroup[] => {
		if (!topology) return [];
		const byType = new SvelteMap<'Host' | 'Service', SvelteSet<string>>();
		for (const node of nodes) {
			const data = node.data as TopologyNode | undefined;
			if (!data) continue;
			const target = resolveTagTarget(node.id, data);
			if (!target) continue;
			let set = byType.get(target.entityType);
			if (!set) {
				set = new SvelteSet();
				byType.set(target.entityType, set);
			}
			set.add(target.entityId);
		}
		const groups: TagGroup[] = [];
		for (const entityType of ['Host', 'Service'] as const) {
			const ids = byType.get(entityType);
			if (!ids || ids.size === 0) continue;
			const idArr = [...ids];
			const source = entityType === 'Host' ? topology.hosts : topology.services;
			const entityList = source.filter((e) => idArr.includes(e.id));
			groups.push({
				entityType,
				entityIds: idArr,
				entities: entityList,
				commonTags: computeCommonTags(entityList),
				label: commonTagsHeader(entityType)
			});
		}
		return groups;
	});

	// Unified edit state — tutorial mode always allows edits
	let editState = $derived(
		isTutorial
			? { isReadonly: false, isEditable: true, disabledReason: null }
			: getTopologyEditState(topology, false, isReadOnly || $topologyReadOnly)
	);

	let mutateDisabledReason = $derived.by(() => {
		if (!editState.disabledReason) return '';
		return topology_multiSelectReadOnlyHint();
	});

	// Merge topology entity_tags with tags query cache for newly created tags
	const tagsQuery = useTagsQuery();
	let topoEntityTags = $derived.by(() => {
		const topoTags = topology?.entity_tags ?? [];
		const cachedTags = tagsQuery.data ?? [];
		const topoIds = new Set(topoTags.map((t) => t.id));
		return [...topoTags, ...cachedTags.filter((t) => !topoIds.has(t.id))];
	});

	let appTagIds = $derived(topoEntityTags.filter((t) => t.is_application).map((t) => t.id));

	let appTagSet = $derived(new Set(appTagIds));

	// Filtered tag lists for pickers
	let nonAppTags = $derived(topoEntityTags.filter((t) => !t.is_application));
	let appTags = $derived(topoEntityTags.filter((t) => t.is_application));

	// Common app tags across selected services (for app picker selectedTagIds).
	// Always derived from services — app-group tagging only applies to services.
	let commonAppTags = $derived(
		computeCommonTags(selectedServices).filter((id) => appTagSet.has(id))
	);
	let hasAppTag = $derived(commonAppTags.length > 0);

	// App-group available tags: if already tagged, only show current tag (for removal)
	let appAvailableTags = $derived(
		hasAppTag ? appTags.filter((t) => commonAppTags.includes(t.id)) : appTags
	);

	// Analyze each selected service's app status
	type AppInfo = { tagId: string; inherited: boolean } | null;

	let serviceAppInfos = $derived.by((): AppInfo[] => {
		if (!topology) return [];
		return selectedServices.map((service) => {
			// Check for direct app tag on the service
			for (const tagId of service.tags) {
				if (appTagSet.has(tagId)) {
					return { tagId, inherited: false };
				}
			}
			// Check for inherited app tag from host
			const host = topology!.hosts.find((h) => h.id === service.host_id);
			if (host) {
				for (const tagId of host.tags) {
					if (appTagSet.has(tagId)) {
						return { tagId, inherited: true };
					}
				}
			}
			return null; // Ungrouped
		});
	});

	// Overall app selection state
	let appState = $derived.by(() => {
		const infos = serviceAppInfos;
		if (infos.length === 0) return { type: 'ungrouped' as const };

		const uniqueTagIds = new Set(infos.map((i) => i?.tagId ?? '__ungrouped__'));
		if (uniqueTagIds.size > 1) return { type: 'cross-group' as const };

		const tagId = infos[0]?.tagId;
		if (!tagId) return { type: 'ungrouped' as const };

		const allInherited = infos.every((i) => i?.inherited === true);
		const allDirect = infos.every((i) => i?.inherited === false);
		return {
			type: 'single' as const,
			tagId,
			allInherited,
			allDirect,
			mixed: !allInherited && !allDirect
		};
	});

	// Get the tag object for the current app
	let currentAppTag = $derived.by(() => {
		if (appState.type !== 'single') return null;
		return topoEntityTags.find((t) => t.id === appState.tagId) ?? null;
	});

	// Track recently added non-app tags for "create grouping rule" action
	let recentlyAddedTagIds = $state<string[]>([]);

	// Per-group tag handlers — one picker per TagGroup drives these with its own entityType/ids.
	// Mutation onSuccess handles cache updates optimistically.
	async function handleAddTagForGroup(group: TagGroup, tagId: string) {
		await bulkAddTagMutation.mutateAsync({
			entity_ids: group.entityIds,
			entity_type: group.entityType,
			tag_id: tagId
		});
		recentlyAddedTagIds = [...recentlyAddedTagIds, tagId];
	}

	async function handleRemoveTagForGroup(group: TagGroup, tagId: string) {
		await bulkRemoveTagMutation.mutateAsync({
			entity_ids: group.entityIds,
			entity_type: group.entityType,
			tag_id: tagId
		});
	}

	// Bindable flag mirroring the app-tag picker's dropdown state.
	let appPickerOpen = $state(false);

	// Tracks whether the user clicked X on the Ungrouped pseudotag in the current session.
	// While true, the pseudotag hides and the picker's "+" affordance is also hidden so the
	// dropdown input is the only visible control. If the user closes the dropdown without
	// picking a tag and the selection is still ungrouped, we restore the pseudotag.
	let ungroupedDismissed = $state(false);

	// Gates the restore-on-close effect so it doesn't fire between the dropdown closing
	// (sync, from handleAddTag in the picker) and the bulk-add mutation resolving.
	let addingAppTag = $state(false);

	// App-group tag handlers — always target the selected services. No rule tracking.
	async function handleAddAppTag(tagId: string) {
		addingAppTag = true;
		try {
			await bulkAddTagMutation.mutateAsync({
				entity_ids: selectedServiceIds,
				entity_type: 'Service',
				tag_id: tagId
			});
		} finally {
			addingAppTag = false;
		}
	}

	async function handleRemoveAppTag(tagId: string) {
		await bulkRemoveTagMutation.mutateAsync({
			entity_ids: selectedServiceIds,
			entity_type: 'Service',
			tag_id: tagId
		});
	}

	$effect(() => {
		if (!appPickerOpen && !addingAppTag && ungroupedDismissed && appState.type === 'ungrouped') {
			ungroupedDismissed = false;
		}
	});

	// Check if a ByTag rule already exists covering the recently added tags
	let existingRuleCoversRecentTags = $derived.by(() => {
		if (recentlyAddedTagIds.length === 0) return false;
		const rules = $topologyOptions?.request?.element_rules ?? [];
		const recentSet = new Set(recentlyAddedTagIds);
		return rules.some((rule) => {
			if (typeof rule.rule === 'object' && 'ByTag' in rule.rule) {
				const ruleTagIds = rule.rule.ByTag.tag_ids;
				return ruleTagIds.some((id: string) => recentSet.has(id));
			}
			return false;
		});
	});

	// Resolve recently added tags to tag objects for display
	let recentlyAddedTags = $derived(
		recentlyAddedTagIds
			.map(
				(id) =>
					topoEntityTags.find((t) => t.id === id) ?? topology?.entity_tags?.find((t) => t.id === id)
			)
			.filter(Boolean)
	);

	function createGroupingRuleFromTags(tagIds: string[]) {
		const names = tagIds
			.map(
				(id) =>
					topoEntityTags.find((t) => t.id === id)?.name ??
					topology?.entity_tags?.find((t) => t.id === id)?.name
			)
			.filter((n): n is string => !!n);
		const title = names.length > 0 ? names.join(', ') : null;
		updateSharedElementRules((current) => [
			...current,
			{
				id: uuidv4(),
				rule: { ByTag: { tag_ids: tagIds, title } }
			}
		]);
		recentlyAddedTagIds = [];
		// `updateSharedElementRules` updates the options store, which the
		// debounced options-store subscription persists (PUTting the raw
		// topology row). No explicit PUT here — spreading the enriched
		// topology would send a flat, view-sliced graph the backend can't
		// deserialize into its per-view node/edge map.
	}

	// Dependency creation state — always visible, no expand/collapse
	function getNodeNames(): string[] {
		return nodes.map((n) => (n.data as TopologyNode)?.header ?? '').filter(Boolean);
	}

	// Edge styling lives outside the TanStack form: EdgeStyleForm manages it via
	// bindable callbacks. Seeded from editingDependency in edit mode (snapshot at
	// open), otherwise a random palette pick.
	// svelte-ignore state_referenced_locally
	let dependencyColor: Color = $state(
		editingDependency?.color ??
			AVAILABLE_COLORS[Math.floor(Math.random() * AVAILABLE_COLORS.length)]
	);
	// svelte-ignore state_referenced_locally
	let dependencyEdgeStyle: EdgeStyle = $state(editingDependency?.edge_style ?? 'Bezier');
	let edgeStyleCollapsed = $state(true);

	// Initial default for dependency type — forwarded to the form's defaults.
	const DEFAULT_DEP_TYPE: DependencyType = 'RequestPath';

	let memberModeOptions = $derived([
		{ value: 'Services', label: dependencies_servicesOnly() },
		{ value: 'Bindings', label: dependencies_withPorts() }
	]);

	// Preview toggle with localStorage persistence
	let showPreview = $state(true);
	if (browser) {
		try {
			const stored = localStorage.getItem(PREVIEW_STORAGE_KEY);
			if (stored !== null) showPreview = stored === 'true';
		} catch {
			// ignore
		}
	}

	function togglePreview() {
		showPreview = !showPreview;
		if (browser) {
			try {
				localStorage.setItem(PREVIEW_STORAGE_KEY, String(showPreview));
			} catch {
				// ignore
			}
		}
		if (!showPreview) {
			previewEdges.set([]);
		} else {
			updatePreviewEdges();
		}
	}

	// Fake dependency data for EdgeStyleForm binding
	let edgeStyleFormData = $derived({
		color: dependencyColor,
		edge_style: dependencyEdgeStyle,
		id: '',
		name: '',
		description: '',
		members: { type: 'Services' as const, service_ids: [] as string[] },
		created_at: '',
		updated_at: '',
		dependency_type: DEFAULT_DEP_TYPE,
		source: { type: 'Manual' as const },
		site_id: '',
		tags: []
	});

	// ----- Element-aware dependency creation -----

	// In edit mode, the X-button adds services here so they get filtered out of depTargets.
	const removedServiceIds = new SvelteSet<string>();

	// Resolve selected nodes to dependency targets (service / host / ipAddress).
	// Each non-service target requires the user to pick its services.
	let depTargets = $derived<DependencyTarget[]>(
		(() => {
			if (!topology) return [];
			if (editingDependency) {
				return resolveEditDependencyTargets(editingDependency, nodes, topology, removedServiceIds);
			}
			return resolveDependencyTargets(nodes, topology);
		})()
	);

	function removeTarget(target: DependencyTarget) {
		if (editingDependency && target.type === 'service') {
			removedServiceIds.add(target.serviceId);
		}
		// Also drop the node from the canvas selection so the highlight goes away —
		// in edit mode startEditing() pushed the dep's member services into
		// selectedNodes, so the canvas ring stays until we prune here.
		selectedNodes.update((ns) => ns.filter((n) => n.id !== target.elementId));
	}

	// L3 (or any view marking Bindings required) forces the binding toggle on.
	let bindingsRequired = $derived(inspectorConfig.dependency_creation === 'Bindings');

	// Single TanStack form. `picks` is a single-select per multi-candidate target
	// (`picks.<elementId> = <chosen serviceId>`); `bindings.<serviceId> = <bindingId>`.
	// Service / single-candidate targets don't need form entries — derived resolution
	// knows their service ID directly.
	type MemberMode = 'Services' | 'Bindings';
	interface DepFormValues {
		name: string;
		dependency_type: DependencyType;
		memberMode: MemberMode;
		picks: Record<string, string>;
		bindings: Record<string, string>;
	}

	// Each card's resolved service, in canvas selection order. elementId is the
	// canvas node ID (unique per card) — the form's bindings map is keyed by
	// elementId so multiple cards that resolve to the same serviceId don't race
	// on a shared field and cause reactive loops.
	interface ResolvedService {
		elementId: string;
		serviceId: string;
		ipAddressIdFilter: string | null;
	}

	function buildInitialFormValues(): DepFormValues {
		if (editingDependency) {
			// In edit mode, each dep member maps to a service-type target whose
			// elementId equals its serviceId — so seeding bindings by serviceId is
			// equivalent to seeding by elementId here.
			const bindingsSeed: Record<string, string> = {};
			if (editingDependency.members.type === 'Bindings' && topology) {
				for (const bid of editingDependency.members.binding_ids) {
					const svc = topology.services.find((s) => s.bindings.some((b) => b.id === bid));
					if (svc) bindingsSeed[svc.id] = bid;
				}
			}
			return {
				name: editingDependency.name,
				dependency_type: editingDependency.dependency_type,
				memberMode: editingDependency.members.type,
				picks: {},
				bindings: bindingsSeed
			};
		}
		return {
			name: '',
			dependency_type: DEFAULT_DEP_TYPE,
			memberMode: 'Services',
			picks: {},
			bindings: {}
		};
	}

	const form = createForm(() => ({
		defaultValues: buildInitialFormValues() as DepFormValues,
		onSubmit: async ({ value }) => {
			if (!topology) return;
			const v = value as DepFormValues;

			let members: Dependency['members'];
			if (v.memberMode === 'Bindings') {
				const bindingIds: string[] = [];
				for (const r of resolvedServices) {
					// Try elementId first (new keying). Fall back to serviceId for edit-mode
					// seeds written by buildInitialFormValues, where elementId === serviceId.
					const id = v.bindings?.[r.elementId] ?? v.bindings?.[r.serviceId];
					if (!id) return;
					bindingIds.push(id);
				}
				members = { type: 'Bindings', binding_ids: bindingIds };
			} else {
				members = {
					type: 'Services',
					service_ids: resolvedServices.map((r) => r.serviceId)
				};
			}

			if (editingDependency) {
				const updated: Dependency = {
					...editingDependency,
					name: v.name.trim(),
					dependency_type: v.dependency_type,
					color: dependencyColor,
					edge_style: dependencyEdgeStyle,
					members
				};
				await updateDependencyMutation.mutateAsync(updated);
				previewEdges.set([]);
				onDone?.();
			} else {
				const newDependency = createEmptyDependencyFormData(topology.site_id);
				newDependency.name = v.name.trim();
				newDependency.dependency_type = v.dependency_type;
				newDependency.color = dependencyColor;
				newDependency.edge_style = dependencyEdgeStyle;
				newDependency.members = members;

				const created = await createDependencyMutation.mutateAsync(newDependency);
				previewEdges.set([]);
				onGroupCreated?.(created.id);
			}
		}
	}));

	// TanStack Form's `form.state.values` is not tracked by Svelte 5 $derived.
	// Mirror it into a $state via form.store.subscribe (pattern from CreateDaemonModal).
	let formValues = $state<DepFormValues>({
		name: '',
		dependency_type: DEFAULT_DEP_TYPE,
		memberMode: 'Services',
		picks: {},
		bindings: {}
	});
	$effect(() => {
		return form.store.subscribe(() => {
			const v = form.state.values as DepFormValues;
			formValues = {
				name: v.name ?? '',
				dependency_type: v.dependency_type ?? DEFAULT_DEP_TYPE,
				memberMode: v.memberMode ?? 'Services',
				picks: { ...(v.picks ?? {}) },
				bindings: { ...(v.bindings ?? {}) }
			};
		});
	});

	let resolvedServices = $derived<ResolvedService[]>(
		(() => {
			if (!topology) return [] as ResolvedService[];
			const out: ResolvedService[] = [];
			const picksMap = formValues.picks;
			for (const target of depTargets) {
				if (target.type === 'service') {
					out.push({
						elementId: target.elementId,
						serviceId: target.serviceId,
						ipAddressIdFilter: null
					});
				} else {
					if (target.candidateServiceIds.length === 0) continue;
					const picked =
						target.candidateServiceIds.length === 1
							? target.candidateServiceIds[0]
							: (picksMap[target.elementId] ?? target.candidateServiceIds[0]);
					out.push({
						elementId: target.elementId,
						serviceId: picked,
						ipAddressIdFilter: target.type === 'ipAddress' ? target.ipAddressId : null
					});
				}
			}
			return out;
		})()
	);

	let allServicesHaveBindings = $derived(
		resolvedServices.length > 0 &&
			resolvedServices.every(
				(r) => !!formValues.bindings[r.elementId] || !!formValues.bindings[r.serviceId]
			)
	);

	// Keep the view-driven "bindings required" flag in sync with the form.
	$effect(() => {
		if (bindingsRequired && formValues.memberMode !== 'Bindings') {
			form.setFieldValue('memberMode', 'Bindings');
		}
	});

	// Bubble dependency_type changes to the parent (tutorial checklist watches this).
	// Skip the initial mount firing — only notify on real user-driven changes.
	let previousDependencyType: DependencyType | undefined;
	$effect(() => {
		const current = formValues.dependency_type;
		if (previousDependencyType !== undefined && previousDependencyType !== current) {
			onDependencyTypeChange?.(current);
		}
		previousDependencyType = current;
	});

	let lastAutoName = $state('');
	// Auto-generate dependency name when type or selection changes, unless the user edited it.
	$effect(() => {
		const newName = generateDependencyName(formValues.dependency_type, getNodeNames());
		if (
			(formValues.name === '' || formValues.name === lastAutoName) &&
			formValues.name !== newName
		) {
			form.setFieldValue('name', newName);
		}
		lastAutoName = newName;
	});

	// Any host / IPAddress target with zero candidate services is invalid — its
	// card shows an inline error and submit is blocked until the user removes it.
	let hasTargetWithoutServices = $derived(
		depTargets.some((t) => t.type !== 'service' && t.candidateServiceIds.length === 0)
	);

	let canCreate = $derived.by(() => {
		if (!formValues.name.trim()) return false;
		if (createDependencyMutation.isPending || updateDependencyMutation.isPending) return false;
		if (resolvedServices.length < 2) return false;
		if (hasTargetWithoutServices) return false;
		if (formValues.memberMode === 'Bindings' && !allServicesHaveBindings) return false;
		return true;
	});

	async function confirmGroupCreation() {
		if (!canCreate) return;
		await submitForm(form);
	}

	function cancelEdit() {
		previewEdges.set([]);
		onDone?.();
	}

	// Absolute position from xyflow's own rendered state — accounts for
	// every level of nesting, zoom/pan, and user drag. Fall back to the
	// raw node position only if the node isn't mounted yet (e.g. first
	// render before xyflow has measured it).
	function getAbsolutePosition(node: Node): { x: number; y: number } {
		const internal = getInternalNode(node.id);
		const abs = internal?.internals.positionAbsolute;
		if (abs) return { x: abs.x, y: abs.y };
		return { x: node.position.x, y: node.position.y };
	}

	function nodeSize(node: Node): { w: number; h: number } {
		// Svelte Flow populates measured.{width,height} once the node is laid out.
		const measured = (node as Node & { measured?: { width?: number; height?: number } }).measured;
		return {
			w: measured?.width ?? node.width ?? 0,
			h: measured?.height ?? node.height ?? 0
		};
	}

	function handlesFor(source: Node, target: Node): { sourceHandle: string; targetHandle: string } {
		// Prefer handles from the currently-rendered real edge for this pair
		// (if one exists). That keeps the preview visually identical to the
		// edge it's replacing during an edit, and stays consistent with the
		// build-flow-edges handle-picker. Falls back to computing against
		// xyflow live positions for pairs that have no real edge (new deps
		// or newly-rearranged hub-spoke orderings).
		// Read from the source-of-truth store so we get the real edge's handles
		// even when the merge effect has filtered it out of the rendered `edges`
		// store during the edit.
		const currentEdges = get(baseFlowEdges);
		const matching = currentEdges.find((e) => e.source === source.id && e.target === target.id);
		if (matching?.sourceHandle && matching?.targetHandle) {
			return {
				sourceHandle: matching.sourceHandle,
				targetHandle: matching.targetHandle
			};
		}
		return computeOptimalHandles(
			getAbsolutePosition(source),
			nodeSize(source),
			getAbsolutePosition(target),
			nodeSize(target)
		);
	}

	// Preview edges — render as colored group edges
	function updatePreviewEdges() {
		if (nodes.length < 2) return;

		const colorHelper = createColorHelper(dependencyColor);
		const preview: Edge[] = [];
		const depType = formValues.dependency_type;

		if (depType === 'RequestPath') {
			for (let i = 0; i < nodes.length - 1; i++) {
				const source = nodes[i];
				const target = nodes[i + 1];
				const handles = handlesFor(source, target);
				preview.push({
					id: `preview-${i}`,
					source: source.id,
					target: target.id,
					sourceHandle: handles.sourceHandle,
					targetHandle: handles.targetHandle,
					type: 'custom',
					data: {
						edge_type: 'RequestPath',
						is_preview: true,
						dependency_id: '__preview__',
						preview_color: dependencyColor,
						preview_edge_style: dependencyEdgeStyle
					},
					markerEnd: {
						type: 'arrow',
						color: colorHelper.rgb
					}
				});
			}
		} else {
			const hub = nodes[0];
			for (let i = 1; i < nodes.length; i++) {
				const spoke = nodes[i];
				const handles = handlesFor(hub, spoke);
				preview.push({
					id: `preview-${i}`,
					source: hub.id,
					target: spoke.id,
					sourceHandle: handles.sourceHandle,
					targetHandle: handles.targetHandle,
					type: 'custom',
					data: {
						edge_type: 'HubAndSpoke',
						is_preview: true,
						dependency_id: '__preview__',
						preview_color: dependencyColor,
						preview_edge_style: dependencyEdgeStyle
					},
					markerEnd: {
						type: 'arrow',
						color: colorHelper.rgb
					}
				});
			}
		}
		previewEdges.set(preview);
	}

	// Start preview edges on mount and update when dependencies change
	$effect(() => {
		if (showPreview) {
			void dependencyColor;
			void formValues.dependency_type;
			void dependencyEdgeStyle;
			void nodes;
			updatePreviewEdges();
		}
	});
</script>

<div class="w-full space-y-4">
	<!-- Header with count, focus, and clear -->
	<div class="flex items-center justify-between">
		<span class="text-secondary text-sm font-medium">
			{appWizard_selectedCount({ summary: selectionSummary })}
		</span>
		<div class="flex items-center gap-1">
			{#if !isTutorial}
				<button
					class="btn-icon p-1"
					onclick={() =>
						fitView({ nodes: nodes.map((n) => ({ id: n.id })), padding: 0.5, duration: 300 })}
					title={topology_focusSelection()}
				>
					<Crosshair class="h-4 w-4" />
				</button>
			{/if}
			{#if !isTutorial}
				<button class="btn-icon p-1" onclick={onClearSelection} title={common_clearSelection()}>
					<X class="h-4 w-4" />
				</button>
			{/if}
		</div>
	</div>

	{#if editState.isEditable}
		{#if !isTutorial}
			<!-- Tags sections — one per resolved taggable entity type in the selection -->
			{#each tagGroups as group (group.entityType)}
				{@const nonAppCommon = group.commonTags.filter((id) => !appTagSet.has(id))}
				<div class="space-y-2">
					<span class="text-secondary block text-sm font-medium">{group.label}</span>
					<div class="card card-static space-y-2 p-2">
						{#if nonAppCommon.length === 0 && group.entities.length > 1}
							<p class="text-tertiary text-xs italic">
								{noCommonTagsHintText(group.entityType)}
							</p>
						{/if}
						<div class="flex items-center gap-1.5">
							<TagPickerInline
								selectedTagIds={nonAppCommon}
								onAdd={(tagId) => handleAddTagForGroup(group, tagId)}
								onRemove={(tagId) => handleRemoveTagForGroup(group, tagId)}
								availableTags={nonAppTags}
							/>
						</div>
					</div>
				</div>
			{/each}

			{#if recentlyAddedTagIds.length > 0 && !existingRuleCoversRecentTags}
				<button
					class="btn-secondary flex w-full items-center justify-center gap-1.5 text-xs"
					onclick={() => createGroupingRuleFromTags(recentlyAddedTagIds)}
				>
					<span>{inspector_createGroupingRuleFromTag()}</span>
					{#each recentlyAddedTags as tag (tag?.id)}
						{#if tag}
							<Tag label={tag.name} color={tag.color} />
						{/if}
					{/each}
				</button>
			{/if}

			{#if inspectorConfig.show_application_picker}
				<!-- App-group tag picker — with cross-group and inheritance awareness -->
				<div class="space-y-2">
					<span class="text-secondary block text-sm font-medium">{common_application()}</span>
					<div class="card card-static space-y-2 p-2">
						{#if appState.type === 'cross-group'}
							<p class="text-tertiary text-xs">{tags_crossGroupSelectionHint()}</p>
						{:else}
							{#if appState.type === 'single' && appState.allInherited && currentAppTag}
								<div class="flex flex-wrap items-center gap-1">
									<Tag
										label={currentAppTag.name}
										color={currentAppTag.color}
										icon={concepts.getIconComponent('Application')}
										isShiny={true}
									/>
									<span class="text-tertiary text-xs">{tags_inheritedFromHost()}</span>
								</div>
								<p class="text-tertiary text-xs">{tags_inheritedOverrideHint()}</p>
							{/if}
							<div class="flex flex-wrap items-center gap-1.5">
								{#if appState.type === 'ungrouped' && !ungroupedDismissed}
									<Tag
										label={common_ungrouped()}
										color="Gray"
										icon={concepts.getIconComponent('Application')}
										isShiny={true}
										pill={true}
										removable={true}
										onRemove={() => {
											ungroupedDismissed = true;
											appPickerOpen = true;
										}}
									/>
								{/if}
								<TagPickerInline
									bind:open={appPickerOpen}
									selectedTagIds={commonAppTags}
									onAdd={handleAddAppTag}
									onRemove={handleRemoveAppTag}
									availableTags={appAvailableTags}
									allowCreate={!hasAppTag}
									createAsApplication={true}
									hideAddButton={appState.type === 'ungrouped' && !ungroupedDismissed}
								/>
							</div>
						{/if}
					</div>
				</div>
			{/if}
		{/if}

		{#if inspectorConfig.dependency_creation || isEditMode}
			<!-- Dependency creation / edit — flat layout, no outer wrapping card -->
			<div class="space-y-3">
				<span class="text-secondary block text-sm font-medium"
					>{isEditMode ? dependencies_editDependency() : dependencies_createDependency()}</span
				>

				<!-- Dependency type toggle + preview button -->
				<form.Field name="dependency_type">
					{#snippet children(field)}
						<div class="flex items-center gap-2">
							<SegmentedControl
								options={['RequestPath', 'HubAndSpoke'].map((type) => ({
									value: type,
									label: '',
									icon: dependencyTypes.getIconComponent(type)
								}))}
								selected={field.state.value}
								onchange={(v) => field.handleChange(v as DependencyType)}
							/>
							<span class="text-secondary text-xs"
								>{dependencyTypes.getName(field.state.value)}</span
							>
							<button
								class="btn-secondary ml-auto flex items-center gap-1 p-1.5 text-xs"
								onclick={togglePreview}
								title={showPreview ? topology_hidePreview() : topology_showPreview()}
							>
								{#if showPreview}
									<Eye class="h-3.5 w-3.5" />
								{:else}
									<EyeOff class="h-3.5 w-3.5" />
								{/if}
								{topology_multiSelectPreviewEdge()}
							</button>
						</div>
					{/snippet}
				</form.Field>

				<!-- Name input -->
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field)}
						<TextInput
							label={common_entityName({ entity: common_dependency() })}
							id="dependency-name"
							{field}
							placeholder={common_entityName({ entity: common_dependency() })}
						/>
					{/snippet}
				</form.Field>

				<!-- Edge style & color — collapsed by default; matches how EdgeStyleForm
				     renders on a selected dep edge. -->
				<div class="card card-static p-2">
					<EdgeStyleForm
						bind:formData={edgeStyleFormData}
						bind:collapsed={edgeStyleCollapsed}
						editable={true}
						showCollapseToggle={true}
						onColorChange={(c) => (dependencyColor = c)}
						onEdgeStyleChange={(s) => (dependencyEdgeStyle = s)}
					/>
				</div>

				<!-- Services only / With ports -->
				{#if !isTutorial && !bindingsRequired}
					<form.Field name="memberMode">
						{#snippet children(field)}
							<SegmentedControl
								options={memberModeOptions}
								selected={field.state.value}
								onchange={(v) => field.handleChange(v as MemberMode)}
								size="sm"
								fullWidth={true}
							/>
							{#if $activeView !== 'L3Logical'}
								<p class="text-tertiary mt-1 text-xs">
									{field.state.value === 'Bindings'
										? dependencies_withPortsL3Hint()
										: dependencies_servicesOnlyL3Hint()}
								</p>
							{/if}
						{/snippet}
					</form.Field>
				{/if}

				<!-- Services section: one card per selection, in canvas order.
				     Between-card arrow + direction label:
				       RequestPath: "↓ calls" between every pair of cards.
				       HubAndSpoke: "↓ serves" between Hub card and Spokes header. -->
				{#if topology && depTargets.length > 0}
					{@const depType = formValues.dependency_type}
					<div class="space-y-2">
						{#each depTargets as target, targetIdx (target.elementId)}
							{@const flatIndex = depTargets
								.slice(0, targetIdx)
								.filter((t) => t.type === 'service' || t.candidateServiceIds.length > 0).length}
							{#if depType === 'HubAndSpoke' && targetIdx === 0}
								<span class="text-tertiary block text-xs font-semibold uppercase"
									>{common_hub()}</span
								>
							{:else if depType === 'HubAndSpoke' && targetIdx === 1}
								<div class="flex flex-col items-center gap-0.5">
									<span class="text-tertiary text-xs">{common_serves()}</span>
									<ArrowDown class="text-secondary h-4 w-4" />
								</div>
								<span class="text-tertiary mt-2 block text-xs font-semibold uppercase"
									>{common_spokes()}</span
								>
							{/if}
							<DependencyTargetCard
								{form}
								{topology}
								{target}
								{flatIndex}
								onRemove={depTargets.length > 2 ? () => removeTarget(target) : undefined}
							/>
							{#if depType === 'RequestPath' && targetIdx < depTargets.length - 1}
								<div class="flex flex-col items-center gap-0.5">
									<span class="text-tertiary text-xs">{common_makesRequestTo()}</span>
									<ArrowDown class="text-secondary h-4 w-4" />
								</div>
							{/if}
						{/each}
					</div>
				{/if}

				<!-- Rebuild warning + submit / cancel buttons. In edit mode, show Cancel + Update;
				     in create mode, just Create Dependency (optionally with rebuild warning). -->
				{#if isTutorial}
					<button class="btn-primary w-full text-xs" onclick={onClearSelection}>
						{topology_tutorialDone()}
					</button>
				{:else}
					<div class="space-y-2">
						{#if isEditMode}
							<div class="flex gap-2">
								<button class="btn-secondary flex-1 text-xs" onclick={cancelEdit}>
									{common_cancel()}
								</button>
								<button
									class="btn-primary flex-1 text-xs"
									onclick={confirmGroupCreation}
									disabled={!canCreate}
								>
									{common_update()}
								</button>
							</div>
						{:else}
							<button
								class="btn-primary w-full text-xs"
								onclick={confirmGroupCreation}
								disabled={!canCreate}
							>
								{dependencies_createDependency()}
							</button>
						{/if}
					</div>
				{/if}
			</div>
		{/if}
	{:else}
		<!-- Show reason why actions are unavailable -->
		{#if mutateDisabledReason}
			<div class="text-tertiary py-2 text-center text-sm">
				{mutateDisabledReason}
			</div>
		{/if}
	{/if}
</div>
