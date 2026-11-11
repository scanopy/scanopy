/**
 * Single owner of what a container's header shows: its title, the chip it draws for the entity it
 * stands for, its status pill, its rule pills and, collapsed, its subgroup rows.
 *
 * `ContainerNode.svelte` renders from this and the diagram exports redraw from it, the same
 * arrangement `element-render-data.ts` gives element cards. Add to this function rather than
 * deriving header content in either consumer.
 */

import type { components } from '#lib/api/schema';
import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import type { Site } from '#lib/features/sites/types.ts';
import { tagTooltip } from '#lib/features/tags/groups.ts';
import type { Tag, TagGroup } from '#lib/features/tags/types/base.ts';
import type { TooltipContent } from '#lib/shared/actions/tooltip.ts';
import type { TagProps } from '#lib/shared/components/data/types.ts';
import { entityUIConfig } from '#lib/shared/entity-ui-config.ts';
import { containerTypes, entities, serviceCategories } from '#lib/shared/stores/metadata.ts';
import { getConfirmationTag } from '#lib/shared/utils/confirmation.ts';
import { getFreshnessTag } from '#lib/shared/utils/freshness.ts';
import {
	createColorHelper,
	createIconComponent,
	type Color,
	type ColorStyle
} from '#lib/shared/utils/styling.ts';
import type { IconComponent } from '#lib/shared/utils/types.ts';
import { entityGlyph } from './entity-glyph';
import { formatElementSummary, tallyContainerElements, tallyDirectElements } from './labels';
import { findInfraRuleId, getInfrastructureRuleIdForTopology } from './queries';
import { containerEntity, resolveContainerNode, type NodeEntity } from './resolvers';
import type { RenderableTopology, TopologyNode, TopologyOptions } from './types/base';

/** Flow units a grouping container's external title pill sits above its top edge. */
export const EXTERNAL_TITLE_OFFSET = 40;

type DeviceType = components['schemas']['DeviceType'];
type EntityType = NonNullable<ReturnType<typeof containerEntity>>['entityType'];

/** A group container's pill: a category or tag the group collects, with its tooltip. */
export type GroupPill = { label: string; color: Color; title?: string | TooltipContent };

/** One subgroup of a collapsed root container, as a row under its summary. */
export type SubgroupRow = {
	/** Service definition whose logo marks the subgroup, drawn round as software. */
	logoDefinition: string | null;
	headerText: string;
	labels: GroupPill[];
	childCount: number;
	/** Pre-formatted entity-count summary, e.g. "5 services" or "3 services, 2 hosts". */
	childSummary: string;
	hideCount?: boolean;
	countOnly?: boolean;
};

export interface ContainerRenderInputs {
	nodeId: string;
	node: TopologyNode;
	topology: RenderableTopology;
	activeView: string;
	options: TopologyOptions;
	/** Sites, for resolving the entity's staleness window. */
	sites: Site[];
	isCollapsed: boolean;
}

export interface ContainerRender {
	containerType: string;
	isSubcontainer: boolean;
	/** Tags of the entity the container stands for. */
	tags: string[];
	/**
	 * The entity the container stands for (a host box, a subnet box, a VLAN box, a hypervisor's
	 * service). Grouping containers (categories, tags, stacks) have none.
	 */
	entity: NodeEntity | undefined;
	entityType: EntityType | undefined;
	entitySite: Site | undefined;
	/** That entity's colour, which draws the title as its pill. Null for a grouping. */
	entityColor: Color | null;
	/** The icon every chip for that entity draws (a host's device type). */
	chipIcon: IconComponent | null;
	/** Unconfirmed ahead of Stale, judged on that entity alone, as on element cards. */
	staleTag: TagProps | null;
	headerText: string;
	childSummary: string;
	/** Summary of the elements directly in the container, outside any subgroup. */
	ungroupedSummary: string;
	/** The node's own icon, else the container type's when it fills its icon. */
	iconComponent: IconComponent | null;
	/** Service definition whose logo marks a Hypervisor, ContainerRuntime or Stack box. */
	serviceDefinition: string | null;
	/** The device type a container standing for a device draws as its tile. */
	deviceType: DeviceType | null;
	colorHelper: ColorStyle;
	isInfraRule: boolean;
	/** Category or tag pills of the element rule that made this subcontainer. */
	groupLabels: GroupPill[];
	/** Rows under a collapsed root container's summary; empty otherwise. */
	subgroups: SubgroupRow[];
}

type ElementRule = NonNullable<TopologyOptions['request']['element_rules']>[number];
type NodeExtras = {
	icon?: string;
	color?: string;
	associated_service_definition?: string;
	element_rule_id?: string;
	subgroupSummaries?: { groupId: string; childCount: number }[];
};

/**
 * A tag's name and colour for a grouping pill. The app's tags cache holds every org tag, so a
 * just-added rule tag resolves at once; the topology bundle is the only source in the
 * unauthenticated share viewer.
 */
function tagPill(tagId: string, topology: RenderableTopology): GroupPill {
	const tag =
		queryClient.getQueryData<Tag[]>(queryKeys.tags.all)?.find((t) => t.id === tagId) ??
		topology.entity_tags?.find((t) => t.id === tagId);
	return {
		label: tag?.name ?? tagId,
		color: (tag?.color as Color) ?? 'Gray',
		title:
			tagTooltip(tag, queryClient.getQueryData<TagGroup[]>(queryKeys.tagGroups.all) ?? []) ??
			undefined
	};
}

/** A category group's pill: the category's name and colour, and its description as the tooltip. */
function categoryPill(category: string): GroupPill {
	const { label, color, title } = serviceCategories.getTag(category);
	return { label, color: color ?? 'Gray', title };
}

/** The pills of the categories or tags an element rule groups by. */
function rulePills(
	rule: ElementRule | null | undefined,
	topology: RenderableTopology
): GroupPill[] {
	const spec = rule?.rule;
	if (!spec || typeof spec === 'string') return [];
	if ('ByServiceCategory' in spec) {
		return (spec.ByServiceCategory.categories ?? []).map(categoryPill);
	}
	if ('ByTag' in spec) {
		return (spec.ByTag.tag_ids ?? []).map((tagId: string) => tagPill(tagId, topology));
	}
	return [];
}

export function buildContainerRender(inputs: ContainerRenderInputs): ContainerRender {
	const { nodeId, node, topology, activeView, options, sites } = inputs;
	const extras = node as TopologyNode & NodeExtras;
	const containerType = (node as { container_type?: string }).container_type ?? 'Subnet';
	const meta = containerTypes.getMetadata(containerType);
	const isSubcontainer = meta?.is_subcontainer ?? false;

	const subject = containerEntity(node, topology);
	const entity = subject?.entity;
	const entityType = subject?.entityType;
	const entitySite = sites.find((n) => n.id === entity?.site_id);

	const rules = options.request.element_rules ?? [];
	const ruleById = (id: string | undefined) =>
		id ? (rules.find((r) => r.id === id) ?? null) : null;
	const infraRuleId = findInfraRuleId(rules);
	const isInfraRule = extras.element_rule_id != null && extras.element_rule_id === infraRuleId;

	const glyph = subject ? entityGlyph(subject.entityType, subject.entity, topology) : null;

	const subgroupSummaries = extras.subgroupSummaries ?? [];
	const subgroups: SubgroupRow[] =
		!isSubcontainer && inputs.isCollapsed
			? subgroupSummaries.map((summary) => {
					const groupNode = topology.nodes.find((n) => n.id === summary.groupId) as
						(TopologyNode & NodeExtras) | undefined;
					const ruleId = groupNode?.element_rule_id;
					const isInfra = !!(infraRuleId && ruleId === infraRuleId);
					return {
						logoDefinition: groupNode?.associated_service_definition ?? null,
						headerText: groupNode?.header ?? '',
						labels: isInfra ? [] : rulePills(ruleById(ruleId), topology),
						childCount: summary.childCount,
						childSummary: formatElementSummary(
							tallyContainerElements(summary.groupId, topology),
							activeView
						),
						countOnly: isInfra
					};
				})
			: [];

	return {
		containerType,
		isSubcontainer,
		tags: resolveContainerNode(nodeId, node, topology).tags ?? [],
		entity,
		entityType,
		entitySite,
		entityColor: entityType ? entities.getColorHelper(entityType).color : null,
		chipIcon:
			entity && entityType
				? (entityUIConfig[entityType]?.displayComponent?.getChipIcon?.(entity as never) ??
					entities.getIconComponent(entityType))
				: null,
		staleTag:
			entity && entityType
				? (getConfirmationTag(entity) ??
					getFreshnessTag(entity, entitySite, {
						entityTypeLabel: entities.getName(entityType) || undefined
					}))
				: null,
		headerText: node.header ?? '',
		childSummary: formatElementSummary(
			tallyContainerElements(nodeId, topology, getInfrastructureRuleIdForTopology(topology)),
			activeView
		),
		ungroupedSummary: formatElementSummary(tallyDirectElements(nodeId, topology), activeView),
		iconComponent: extras.icon
			? createIconComponent(extras.icon)
			: meta?.fill_icon
				? containerTypes.getIconComponent(containerType)
				: null,
		serviceDefinition: extras.associated_service_definition ?? null,
		deviceType: glyph?.type === 'device' ? glyph.deviceType : null,
		colorHelper: extras.color
			? createColorHelper(extras.color as Color)
			: containerTypes.getColorHelper(containerType),
		isInfraRule,
		groupLabels: isInfraRule ? [] : rulePills(ruleById(extras.element_rule_id), topology),
		subgroups
	};
}
