import type { Node } from '@xyflow/svelte';
import type { components } from '$lib/api/schema';
import type { RenderableTopology, TopologyNode } from './types/base';
import { entities, serviceDefinitions, views } from '$lib/shared/stores/metadata';
import { lowercasePreservingAcronyms } from '$lib/shared/utils/formatting';
import { tags_entityTags, tags_noCommonTagsHint } from '$lib/paraglide/messages';
import { resolveInlineServiceIds } from './resolvers';
import { getTopologyIndex } from './entity-index';

type Entity = components['schemas']['EntityDiscriminants'];

/** "service" / "host" / "IP address" — lowercase prose form (preserves acronyms). */
function singular(entity: Entity): string {
	const raw =
		entities.getMetadata(entity)?.entity_name_singular ?? entities.getName(entity) ?? entity;
	return lowercasePreservingAcronyms(raw);
}

/** "services" / "hosts" / "IP addresses". */
function plural(entity: Entity): string {
	const raw =
		entities.getMetadata(entity)?.entity_name_plural ?? entities.getName(entity) ?? entity;
	return lowercasePreservingAcronyms(raw);
}

/** Title-case singular (as emitted by backend, e.g. "Host", "IP Address"). */
function singularTitle(entity: Entity): string {
	return entities.getMetadata(entity)?.entity_name_singular ?? entities.getName(entity) ?? entity;
}

/** Title-case plural (as emitted by backend, e.g. "Hosts", "IP Addresses"). */
function pluralTitle(entity: Entity): string {
	return entities.getMetadata(entity)?.entity_name_plural ?? entities.getName(entity) ?? entity;
}

/** Oxford comma join: ["a","b"]→"a and b", ["a","b","c"]→"a, b, and c". */
function joinList(items: string[]): string {
	if (items.length === 0) return '';
	if (items.length === 1) return items[0];
	if (items.length === 2) return `${items[0]} and ${items[1]}`;
	return `${items.slice(0, -1).join(', ')}, and ${items[items.length - 1]}`;
}

/** Lowercase entity list label. Single-entity views can pass count to pick singular form. */
export function formatEntityLabel(entityTypes: Entity[], count?: number): string {
	if (entityTypes.length === 1) {
		return count === 1 ? singular(entityTypes[0]) : plural(entityTypes[0]);
	}
	return joinList(entityTypes.map(plural));
}

/** Title-case entity list label for headings: "Services", "Hosts and Services", "IP Addresses". */
export function formatEntityLabelTitle(entityTypes: Entity[], count?: number): string {
	if (entityTypes.length === 1) {
		return count === 1 ? singularTitle(entityTypes[0]) : pluralTitle(entityTypes[0]);
	}
	return joinList(entityTypes.map(pluralTitle));
}

/** "1 service, 2 hosts" — per-type counts. */
export function formatEntityCounts(counts: Map<Entity, number>): string {
	const parts: string[] = [];
	for (const [entity, n] of counts) {
		parts.push(`${n} ${n === 1 ? singular(entity) : plural(entity)}`);
	}
	return joinList(parts);
}

/** Tally selected nodes by element entity type. Skips containers (node_type === 'Container'). */
export function tallySelection(nodes: Node[]): Map<Entity, number> {
	const counts = new Map<Entity, number>();
	for (const node of nodes) {
		const data = node.data as { element_type?: Entity; node_type?: string } | undefined;
		const entity = data?.element_type;
		if (!entity) continue;
		counts.set(entity, (counts.get(entity) ?? 0) + 1);
	}
	return counts;
}

/** Tally an iterable of element-carrying topology nodes by entity type. */
function tallyByEntityType(nodes: Iterable<TopologyNode>): Map<Entity, number> {
	const counts = new Map<Entity, number>();
	for (const n of nodes) {
		if (n.node_type !== 'Element') continue;
		const entity = (n as { element_type?: Entity }).element_type;
		if (!entity) continue;
		counts.set(entity, (counts.get(entity) ?? 0) + 1);
	}
	return counts;
}

/** Per-entity-type tally of all element nodes nested under `containerId`
 *  (direct + subcontainer children), plus services rendered inside those
 *  elements via inline relationships (L3 services bound to IP elements;
 *  Workloads services on host elements). Delegates traversal to
 *  `getContainerContents` and entity-relationship resolution to
 *  `resolveInlineServiceIds`.
 *
 *  `excludeRuleId` names a service-category rule (callers pass the
 *  infrastructure rule) whose services stay out of the total in both places
 *  they can appear: as elements in the subcontainer that rule produced, and
 *  as services inlined on another element (a VM's SSH under its hypervisor in
 *  Workloads), matched by the rule's own categories. */
export function tallyContainerElements(
	containerId: string,
	topology: RenderableTopology,
	excludeRuleId: string | null = null
): Map<Entity, number> {
	const index = getTopologyIndex(topology);
	const { elementNodeIds } = index.containerContents(containerId);
	const excludedCategories = excludeRuleId
		? ruleServiceCategories(topology, excludeRuleId)
		: new Set<string>();
	const elementNodes: TopologyNode[] = [];
	const countedIds = new Set<string>();
	for (const id of elementNodeIds) {
		const node = index.nodesById.get(id);
		if (!node) continue;
		if (excludeRuleId && inSubcontainerOfRule(node, excludeRuleId, index.nodesById)) continue;
		elementNodes.push(node);
		countedIds.add(id);
	}
	const counts = tallyByEntityType(elementNodes);
	let inlineServiceCount = 0;
	for (const serviceId of resolveInlineServiceIds(countedIds, topology)) {
		const service = index.servicesById.get(serviceId);
		const category = service ? serviceDefinitions.getCategory(service.service_definition) : null;
		if (category && excludedCategories.has(category)) continue;
		inlineServiceCount++;
	}
	if (inlineServiceCount > 0) {
		counts.set('Service', (counts.get('Service') ?? 0) + inlineServiceCount);
	}
	return counts;
}

/** The service categories a `ByServiceCategory` element rule groups, from the topology's options. */
function ruleServiceCategories(topology: RenderableTopology, ruleId: string): Set<string> {
	for (const identified of topology.options?.request?.element_rules ?? []) {
		if (identified.id !== ruleId) continue;
		const rule = identified.rule;
		if (typeof rule === 'object' && 'ByServiceCategory' in rule) {
			return new Set(rule.ByServiceCategory.categories ?? []);
		}
	}
	return new Set();
}

/** Whether `node` sits in a subcontainer that element rule `ruleId` produced. */
function inSubcontainerOfRule(
	node: TopologyNode,
	ruleId: string,
	nodesById: Map<string, TopologyNode>
): boolean {
	const parentId = (node as { container_id?: string }).container_id;
	if (!parentId) return false;
	const parent = nodesById.get(parentId) as { element_rule_id?: string | null } | undefined;
	return parent?.element_rule_id === ruleId;
}

/** Per-entity-type tally of element nodes whose immediate `container_id`
 *  equals `containerId` (i.e. not in any subcontainer), plus services
 *  inlined on those elements. Used for the "ungrouped" summary on
 *  collapsed-root containers. */
export function tallyDirectElements(
	containerId: string,
	topology: RenderableTopology
): Map<Entity, number> {
	const index = getTopologyIndex(topology);
	const directElements = index.directElementsByContainerId.get(containerId) ?? [];
	const directElementIds = new Set<string>(directElements.map((n) => n.id));
	const counts = tallyByEntityType(directElements);
	const inlineServices = resolveInlineServiceIds(directElementIds, topology);
	if (inlineServices.size > 0) {
		counts.set('Service', (counts.get('Service') ?? 0) + inlineServices.size);
	}
	return counts;
}

/** Read a view's collective noun (e.g. Workloads → "workload"). Singular. */
export function getViewCollectiveNoun(viewId: string | null): string | undefined {
	if (!viewId) return undefined;
	return (
		views.getMetadata(viewId) as { element_config?: { collective_noun?: string } } | undefined
	)?.element_config?.collective_noun;
}

/** Format an element summary string for container-header surfaces. When
 *  the view defines a collective noun, emits "{total} {collective_noun}s"
 *  alone — the per-entity breakdown is redundant with the collective total
 *  in those views (e.g. Workloads: every service is already a workload).
 *  Otherwise emits the per-entity breakdown (e.g. "12 IP addresses and
 *  8 services"). The inspector panel's element summary builds its own
 *  breakdown alongside the collective total and does not use this. */
export function formatElementSummary(counts: Map<Entity, number>, viewId: string | null): string {
	const collective = getViewCollectiveNoun(viewId);
	if (collective) {
		const total = [...counts.values()].reduce((s, n) => s + n, 0);
		return `${total} ${total === 1 ? collective : collective + 's'}`;
	}
	return formatEntityCounts(counts);
}

/** "Common {entity} tags" header — takes typed Entity, renders i18n with lowercase plural. */
export function commonTagsHeader(entity: Entity): string {
	return tags_entityTags({ entity: plural(entity) });
}

/** "Selected {entity} have no tags in common. …" hint for empty common-tags picker. */
export function noCommonTagsHintText(entity: Entity): string {
	return tags_noCommonTagsHint({ entity: plural(entity) });
}
