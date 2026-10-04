/**
 * Single owner of what an element card renders — and therefore of how tall it is.
 *
 * This computation used to live inside `ElementNode.svelte` as a `$derived.by`.
 * It never depended on component-local state: every input is either the node
 * itself or a module-level store, so it was always a pure function that merely
 * happened to live in a component.
 *
 * Moving it out matters because the render pipeline needs to reason about card
 * height *without* rendering the card. The measure pass currently mounts every
 * node just to read its height; to measure only one representative per distinct
 * card shape, something has to decide which cards share a shape. If that
 * decision were made by a second implementation, the two would drift and the
 * layout would silently corrupt. With one function, the shape key is derived
 * from the same result the component renders.
 *
 * Consequently: **anything that affects rendered height must be reflected in
 * `ElementRenderResult`.** Height-affecting markup driven by something outside
 * this function's inputs will not be visible to the shape key.
 */

import type { components } from '$lib/api/schema';
import type { Service } from '$lib/features/services/types/base';
import type {
	ElementInlineGroup,
	ElementRenderData,
	RenderableTopology,
	TopologyNode,
	TopologyOptions
} from './types/base';
import { elementEntity, resolveElementNode } from './resolvers';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import { getTopologyIndex } from './entity-index';
import { entities, serviceDefinitions, views } from '$lib/shared/stores/metadata';
import { getFreshnessTag } from '$lib/shared/utils/freshness';
import type { Network } from '$lib/features/networks/types';
import { get } from 'svelte/store';
import { activeView, topologyOptions } from './queries';
import { collapsedInlineGroups, hiddenEntityIds, inlineGroupKey } from './interactions';
import { queryClient, queryKeys } from '$lib/api/query-client';

type InlineGroup = components['schemas']['InlineGroup'];

/**
 * Whether the active view inlines services / ports on this element, and whether
 * the user has hidden either. These gate whole blocks of card content, so they
 * belong with the render data rather than being recomputed by the template.
 */
export interface ElementInlineFlags {
	/** Entity types the active view inlines on this element (e.g. Service, Port). */
	inlineEntities: string[];
	inlinesService: boolean;
	inlinesPort: boolean;
	serviceInlineHidden: boolean;
	portInlineHidden: boolean;
}

export interface ElementRenderResult {
	data: ElementRenderData | null;
	flags: ElementInlineFlags;
	/**
	 * Staleness pill, or null. Rendered in flow at the top of the card, so its
	 * presence changes the card's height — which is why it belongs here and not
	 * in the component.
	 */
	staleTag: ReturnType<typeof getFreshnessTag>;
}

export interface ElementRenderInputs {
	nodeId: string;
	node: TopologyNode;
	topology: RenderableTopology;
	activeView: string;
	options: TopologyOptions;
	/** Ids of entities hidden by any filter, of any type (`hiddenEntityIds`). */
	hiddenEntityIds: Set<string>;
	/** Manager boxes the viewer collapsed, keyed by `inlineGroupKey`. */
	collapsedInlineGroups: Set<string>;
	/** Networks, for resolving each entity's staleness window. */
	networks: Network[];
}

type ViewElementConfig = {
	element_config?: {
		container_entity?: string;
		element_entities?: Array<{ entity_type: string; inline_entities: string[] }>;
	};
} | null;

function viewConfigFor(activeView: string): ViewElementConfig {
	return views.getMetadata(activeView) as ViewElementConfig;
}

/** Service categories the user hid via the metadata filter, for this view. */
function hiddenServiceCategories(options: TopologyOptions, activeView: string): string[] {
	const byView = (options.request.hide_metadata_values ?? {}) as Record<
		string,
		Record<string, Record<string, string[]>>
	>;
	return byView[activeView]?.['Service']?.['Category'] ?? [];
}

export function elementInlineFlags(
	inputs: Pick<ElementRenderInputs, 'activeView' | 'options'>,
	elementType: string | undefined
): ElementInlineFlags {
	const inlineEntities =
		viewConfigFor(inputs.activeView)?.element_config?.element_entities?.find(
			(e) => e.entity_type === elementType
		)?.inline_entities ?? [];

	// Entity types the user has hidden in this view via the filter panel's eye
	// toggle. (Element/container-level hiding is applied upstream via
	// tagHiddenNodeIds.)
	const hiddenEntities =
		((inputs.options.request.hide_entities ?? {}) as Record<string, string[]>)[inputs.activeView] ??
		[];

	return {
		inlineEntities,
		inlinesService: inlineEntities.includes('Service'),
		inlinesPort: inlineEntities.includes('Port'),
		serviceInlineHidden: hiddenEntities.includes('Service'),
		portInlineHidden: hiddenEntities.includes('Port')
	};
}

/**
 * Staleness pill for the entity this card depicts.
 *
 * Judged on the node's own entity with no inheritance from its host — the same
 * rule the inventory cards apply, so the two surfaces agree and the tooltip's
 * timestamp can never contradict the tag. Type names come from the entity
 * metadata fixture so they stay localized and in step with the backend.
 */
function resolveStaleTag(
	resolved: ReturnType<typeof resolveElementNode>,
	networks: Network[]
): ReturnType<typeof getFreshnessTag> {
	const subject = elementEntity(resolved);
	if (!subject) return null;
	return getFreshnessTag(
		subject,
		networks.find((n) => n.id === subject.network_id),
		{ entityTypeLabel: entities.getName(resolved.elementType ?? 'Host') || undefined }
	);
}

export function buildElementRender(inputs: ElementRenderInputs): ElementRenderResult {
	const { nodeId, node, topology, activeView, options, hiddenEntityIds } = inputs;

	const resolved = resolveElementNode(nodeId, node, topology);
	const flags = elementInlineFlags(inputs, resolved.elementType);

	const elementType = resolved.elementType ?? 'Interface';
	const host = resolved.host;
	const staleTag = resolveStaleTag(resolved, inputs.networks);
	const ipAddress = resolved.ipAddress ?? null;
	const servicesForHost = resolved.services ?? [];

	// Service elements: simpler rendering — single service with host name.
	// Intentionally does NOT read the hidden-category set here — category/tag
	// fading is handled by shouldFadeOut via the hidden-services store, so
	// category toggles don't trigger a recomputation.
	if (elementType === 'Service') {
		const service = resolved.services[0];
		// Hide hostname in views where Host is the container — it's redundant
		const showHostname = viewConfigFor(activeView)?.element_config?.container_entity !== 'Host';
		return {
			flags,
			staleTag,
			data: {
				elementType,
				footerText: null,
				services: service ? [service] : [],
				hiddenOpenPorts: [],
				headerText: showHostname && host ? hostDisplayName(host) : null,
				bodyText: service ? null : 'Unknown Service',
				showServices: !!service,
				isVirtualized: false,
				isContainerized: service?.virtualization_service_id != null,
				isCategoryHidden: false,
				ip_address_id: nodeId,
				inlineGroups: []
			} as ElementRenderData
		};
	}

	// Host elements: show host name with services
	if (elementType === 'Host') {
		if (!host || !resolved.hostId) return { data: null, flags, staleTag };

		const hiddenCategories = hiddenServiceCategories(options, activeView);

		// Services visible in card. Filter = structural remove: hidden services
		// are dropped from the list entirely, not faded. The OpenPorts-category
		// subset is routed to the collapsed "+N open ports" indicator below.
		const isShown = (s: Service) => {
			if (hiddenEntityIds.has(s.id)) return false;
			const category = serviceDefinitions.getCategory(s.service_definition);
			if (category === 'OpenPorts' && hiddenCategories.includes(category)) return false;
			return true;
		};
		const inlineGroups = buildInlineGroups(
			node,
			servicesForHost,
			topology,
			hiddenEntityIds,
			isShown,
			(groupId) => inputs.collapsedInlineGroups.has(inlineGroupKey(nodeId, groupId))
		);
		const groupedServiceIds = new Set(
			inlineGroups.flatMap((g) => [...(g.header ? [g.header] : []), ...g.services]).map((s) => s.id)
		);
		const servicesOnHost = servicesForHost.filter(
			(s) => isShown(s) && !groupedServiceIds.has(s.id)
		);

		// OpenPorts hidden by category → collapsed indicator.
		// (Tag-hidden services of any category are already removed above.)
		const hiddenOpenPorts = servicesForHost.filter((s) => {
			if (hiddenEntityIds.has(s.id)) return false;
			const category = serviceDefinitions.getCategory(s.service_definition);
			return category === 'OpenPorts' && hiddenCategories.includes(category);
		});

		// Service names and port lines hide independently. Render the services
		// block if the view declares EITHER inlined and the user hasn't hidden
		// it — so toggling Services off still leaves port lines visible.
		const showServices =
			((flags.inlinesService && !flags.serviceInlineHidden) ||
				(flags.inlinesPort && !flags.portInlineHidden)) &&
			(servicesOnHost.length !== 0 || hiddenOpenPorts.length !== 0 || inlineGroups.length !== 0);

		const hostLabel = node.header ?? hostDisplayName(host);

		return {
			flags,
			staleTag,
			data: {
				elementType,
				footerText: null,
				services: servicesOnHost,
				hiddenOpenPorts,
				headerText: hostLabel,
				bodyText: showServices ? null : hostLabel,
				showServices,
				isVirtualized: host.virtualization_service_id != null,
				isContainerized: false,
				ip_address_id: nodeId,
				inlineGroups
			} as ElementRenderData
		};
	}

	// Port elements: show port name + status/MAC info
	if (elementType === 'Interface') {
		const ifEntryId =
			'interface_id' in (node as Record<string, unknown>)
				? ((node as Record<string, unknown>).interface_id as string)
				: undefined;
		const iface = ifEntryId ? getTopologyIndex(topology).interfacesById.get(ifEntryId) : undefined;

		let speed: string | null = null;
		if (iface?.speed_bps) {
			const bps = iface.speed_bps;
			if (bps >= 1_000_000_000) speed = `${(bps / 1_000_000_000).toFixed(0)}G`;
			else if (bps >= 1_000_000) speed = `${(bps / 1_000_000).toFixed(0)}M`;
			else speed = `${bps} bps`;
		}

		return {
			flags,
			staleTag,
			data: {
				elementType,
				headerText: node.header ?? null,
				footerText: null,
				bodyText: null,
				showServices: false,
				isVirtualized: false,
				isContainerized: false,
				services: [],
				hiddenOpenPorts: [],
				ip_address_id: '',
				inlineGroups: [],
				portStatus: iface
					? {
							operStatus: iface.oper_status,
							speed,
							macAddress: iface.mac_address ?? null
						}
					: undefined
			} as ElementRenderData
		};
	}

	// IPAddress elements
	if (!host || !resolved.hostId) return { data: null, flags, staleTag };

	const hiddenCategories = hiddenServiceCategories(options, activeView);

	const isContainerSubnet = ipAddress
		? getTopologyIndex(topology).subnetsById.get(ipAddress.subnet_id)?.cidr === '0.0.0.0/0'
		: false;

	// All services bound to this interface (after tag filtering)
	const allServicesOnIPAddress = servicesForHost.filter((s) =>
		s.bindings.some(
			(b) => b.ip_address_id == null || (ipAddress && b.ip_address_id == ipAddress.id)
		)
	);

	// Filter = structural remove (see Host branch for context).
	const servicesOnIPAddress = allServicesOnIPAddress.filter((s) => {
		if (hiddenEntityIds.has(s.id)) return false;
		const category = serviceDefinitions.getCategory(s.service_definition);
		if (category === 'OpenPorts' && hiddenCategories.includes(category)) return false;
		return true;
	});

	const hiddenOpenPorts = allServicesOnIPAddress.filter((s) => {
		if (hiddenEntityIds.has(s.id)) return false;
		const category = serviceDefinitions.getCategory(s.service_definition);
		return category === 'OpenPorts' && hiddenCategories.includes(category);
	});

	const headerText: string | null = node.header ?? null;
	// Service names and port lines hide independently — see the Host branch.
	const showServices =
		((flags.inlinesService && !flags.serviceInlineHidden) ||
			(flags.inlinesPort && !flags.portInlineHidden)) &&
		(servicesOnIPAddress.length != 0 || hiddenOpenPorts.length != 0);

	const subtitleText =
		ipAddress && !isContainerSubnet
			? (ipAddress.name ? ipAddress.name + ': ' : '') + ipAddress.ip_address
			: null;

	return {
		flags,
		staleTag,
		data: {
			elementType,
			footerText: null,
			subtitleText,
			services: servicesOnIPAddress,
			hiddenOpenPorts,
			headerText,
			bodyText: showServices ? null : hostDisplayName(host),
			showServices,
			isVirtualized:
				headerText?.startsWith('Docker @') || isContainerSubnet
					? false
					: host.virtualization_service_id != null,
			isContainerized: false,
			ip_address_id: resolved.ipAddressId ?? '',
			inlineGroups: []
		} as ElementRenderData
	};
}

/**
 * The manager groups the backend inlined on a host card, resolved to entities.
 *
 * Each `inline_groups` entry says which entity type it names: service members come from the
 * card's own services, host members (macvlan containers, a guest's network identities) from the
 * topology's hosts, each carrying the services on that host. Hidden hosts and services drop out.
 */
export function buildInlineGroups(
	node: TopologyNode,
	servicesForHost: Service[],
	topology: RenderableTopology,
	hiddenEntityIds: Set<string>,
	isShown: (s: Service) => boolean,
	isCollapsed: (groupId: string) => boolean = () => false
): ElementInlineGroup[] {
	const entries = ((node as { inline_groups?: InlineGroup[] }).inline_groups ??
		[]) as InlineGroup[];
	if (entries.length === 0) return [];

	const index = getTopologyIndex(topology);
	const groups = new Map<string, ElementInlineGroup>();
	for (const entry of entries) {
		let group = groups.get(entry.group_id);
		if (!group) {
			group = {
				groupId: entry.group_id,
				collapsed: isCollapsed(entry.group_id),
				header: null,
				services: [],
				hosts: []
			};
			groups.set(entry.group_id, group);
		}
		if (entry.entity_type === 'Host') {
			const host = index.hostsById.get(entry.entity_id);
			if (!host || hiddenEntityIds.has(host.id)) continue;
			group.hosts.push({
				host,
				services: (index.servicesByHostId.get(host.id) ?? []).filter(isShown)
			});
			continue;
		}
		const service = servicesForHost.find((s) => s.id === entry.entity_id);
		if (!service || !isShown(service)) continue;
		if (entry.role === 'Header') group.header = service;
		else group.services.push(service);
	}

	return [...groups.values()].filter(
		(g) => g.header || g.services.length > 0 || g.hosts.length > 0
	);
}

/**
 * What the inspector's Services section shows for an element: the element's own services, and
 * the manager boxes its card draws (the same `buildInlineGroups` result, so the two agree on
 * members, hiding and service-less hosts).
 *
 * `ipAddressId` narrows the own services to those bound to that address, for an IP-address
 * element. A Host element passes null and keeps every service on the host.
 */
export function inspectorServiceSections(
	node: TopologyNode,
	services: Service[],
	topology: RenderableTopology,
	hiddenEntityIds: Set<string>,
	ipAddressId: string | null
): { own: Service[]; groups: ElementInlineGroup[] } {
	const isShown = (s: Service) => !hiddenEntityIds.has(s.id);
	const groups = buildInlineGroups(node, services, topology, hiddenEntityIds, isShown);
	const grouped = new Set(
		groups.flatMap((g) => [...(g.header ? [g.header] : []), ...g.services]).map((s) => s.id)
	);
	const own = services.filter(
		(s) =>
			isShown(s) &&
			!grouped.has(s.id) &&
			(ipAddressId === null ||
				s.bindings.some((b) => b.ip_address_id === ipAddressId || b.ip_address_id === null))
	);
	return { own, groups };
}

/**
 * A key identifying cards that render to the same height.
 *
 * Two element nodes whose keys match are assumed to measure identically, which
 * is what lets the pipeline mount one representative per key instead of every
 * node.
 *
 * Derived structurally from `ElementRenderResult` — counts, presence flags and
 * text-length buckets — rather than from a hand-picked list of fields, so a new
 * field added to the render data participates by default instead of being
 * silently ignored.
 *
 * Text contributes a *bucket*, not its content: cards are fixed-width, so a
 * long label wraps to another line while a short one does not. Bucketing by
 * rough length approximates the wrap point without making every distinct string
 * its own shape (which would defeat the sampling entirely).
 *
 * This is a heuristic, and the verification mode in the measure pass exists to
 * catch it being wrong: it re-measures nodes and reports any whose actual
 * height disagrees with the height predicted for their key.
 */
export function elementShapeKey(result: ElementRenderResult): string {
	const d = result.data;
	if (!d) return 'null';

	// ~28 characters fit on a line at the fixed card width; bucket by line count
	// rather than raw length so near-identical labels share a key.
	const lines = (text: string | null | undefined): number =>
		text ? Math.ceil(text.length / 28) : 0;
	const serviceShape = (s: Service): string =>
		`${lines(s.name)}:${s.bindings.filter((b) => b.type === 'Port').length}:${s.service_definition ?? ''}`;

	const parts: (string | number)[] = [
		d.elementType,
		lines(d.headerText),
		lines(d.subtitleText),
		lines(d.bodyText),
		lines(d.footerText),
		d.showServices ? 1 : 0,
		d.isVirtualized ? 1 : 0,
		d.isContainerized ? 1 : 0,
		d.hiddenOpenPorts.length,
		result.staleTag ? 1 : 0,
		// Not just presence: the block renders status, speed and MAC on separate
		// lines, so two interface cards can both "have a port status" and still
		// differ in height. Verification caught exactly this — 1600 synthetic
		// interfaces without MACs measured 54px against 56 demo interfaces with
		// them at 70px, under one key.
		d.portStatus
			? `p${d.portStatus.operStatus ? 1 : 0}${d.portStatus.speed ? 1 : 0}${d.portStatus.macAddress ? 1 : 0}`
			: 'p-',
		// Each service renders its own row, and a row's height depends on its
		// name length and how many port lines it carries.
		result.flags.inlinesService && !result.flags.serviceInlineHidden ? 1 : 0,
		result.flags.inlinesPort && !result.flags.portInlineHidden ? 1 : 0,
		d.services.length,
		// `service_definition` decides whether the row renders a definition icon,
		// and an icon row is not the same height as a bare text row. Keying on the
		// id rather than on "has an icon" over-discriminates a little — two
		// different definitions that both render an icon get separate keys — which
		// is the safe direction: an extra key costs one more measured node, a
		// missed distinction lays cards out at the wrong height.
		...d.services.map(serviceShape),
		// Each group is a dashed box with a header row, its service rows, and per member host a
		// name row followed by that host's service rows.
		d.inlineGroups.length,
		// A collapsed box renders only its header row, so its members don't affect height.
		...d.inlineGroups.map((g) =>
			g.collapsed
				? `gc${g.header ? lines(g.header.name) : 0}`
				: `g${g.header ? lines(g.header.name) : 0}[${g.services.map(serviceShape).join(',')}]` +
					`[${g.hosts.map((h) => `${lines(hostDisplayName(h.host))}(${h.services.map(serviceShape).join(',')})`).join(',')}]`
		)
	];

	return parts.join('|');
}

/**
 * Assemble the non-node inputs from the current store state.
 *
 * The render pipeline needs to compute shape keys outside a component, where
 * the `$store` shorthand is unavailable. Reading them in one place keeps the
 * pipeline and the component agreed on what "current" means.
 */
export function currentElementRenderContext(): Omit<
	ElementRenderInputs,
	'nodeId' | 'node' | 'topology'
> {
	return {
		activeView: get(activeView),
		options: get(topologyOptions),
		hiddenEntityIds: get(hiddenEntityIds),
		collapsedInlineGroups: get(collapsedInlineGroups),
		networks: queryClient.getQueryData<Network[]>(queryKeys.networks.all) ?? []
	};
}
