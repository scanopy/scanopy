<script lang="ts">
	import { Edit, Trash2 } from 'lucide-svelte';
	import GenericCard from '$lib/shared/components/data/GenericCard.svelte';
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import type { Service } from '../types/base';
	import type { Host, Interface, Port } from '$lib/features/hosts/types/base';
	import { formatPort } from '$lib/shared/utils/formatting';
	import { formatInterface } from '$lib/features/hosts/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import { useInterfacesQuery } from '$lib/features/interfaces/queries';
	import { usePortsQuery } from '$lib/features/ports/queries';
	import { SvelteMap } from 'svelte/reactivity';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import {
		common_delete,
		common_edit,
		common_interfaceBindings,
		common_notAssigned,
		common_portBindings,
		common_tags,
		common_unbound
	} from '$lib/paraglide/messages';

	// TanStack Query hooks
	const subnetsQuery = useSubnetsQuery();
	const interfacesQuery = useInterfacesQuery();
	const portsQuery = usePortsQuery();

	// Derived data from queries
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let interfacesData = $derived(interfacesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	interface Props {
		service: Service;
		host: Host;
		onDelete?: (service: Service) => void;
		onEdit?: (service: Service) => void;
		viewMode: 'card' | 'list';
		selected: boolean;
		onSelectionChange?: (selected: boolean) => void;
	}

	let {
		service,
		host,
		onDelete,
		onEdit,
		viewMode,
		selected,
		onSelectionChange = () => {}
	}: Props = $props();

	// Get ports and interfaces from query data for display
	let groupedPortBindings = $derived(
		(() => {
			const portBindings = service.bindings.filter((b) => b.type === 'Port');
			const grouped = new SvelteMap<string | null, { iface: Interface | null; ports: Port[] }>();

			for (const binding of portBindings) {
				const port = portsData.find((p) => p.id === binding.port_id);
				if (!port) continue;

				const interfaceId = binding.interface_id ?? null;
				if (!grouped.has(interfaceId)) {
					const iface = interfaceId ? interfacesData.find((i) => i.id === interfaceId) : null;
					grouped.set(interfaceId, { iface: iface ?? null, ports: [] });
				}
				grouped.get(interfaceId)!.ports.push(port);
			}

			return Array.from(grouped.values()).map(({ iface, ports }) => {
				const portList = ports.map((p) => formatPort(p)).join(', ');
				const label = iface
					? `${iface.name ? iface.name + ': ' : ''} ${iface.ip_address} (${portList})`
					: `${common_unbound()} (${portList})`;
				return {
					id: iface?.id ?? 'unbound',
					label,
					color: entities.getColorHelper('Port').color
				};
			});
		})()
	);

	// Get interface bindings - look up interfaces from query data
	let ifaces = $derived(
		(() => {
			const interfaceBindingIds = service.bindings
				.filter((b) => b.type === 'Interface')
				.map((b) => b.interface_id)
				.filter((id): id is string => id !== null);

			return interfaceBindingIds
				.map((id) => interfacesData.find((i) => i.id === id))
				.filter((i): i is Interface => i !== undefined);
		})()
	);

	// Build card data
	let cardData = $derived({
		title: service.name,
		iconColor: serviceDefinitions.getColorHelper(service.service_definition).icon,
		Icon: serviceDefinitions.getIconComponent(service.service_definition),
		fields: [
			{
				label: 'Host',
				value: [
					{
						id: host.id,
						label: host.name,
						color: entities.getColorHelper('Host').color,
						entityRef: entityRef('Host', host.id, host)
					}
				]
			},
			{
				label: common_portBindings(),
				value: groupedPortBindings,
				emptyText: common_notAssigned()
			},
			{
				label: common_interfaceBindings(),
				value: ifaces.map((iface: Interface) => ({
					id: iface.id,
					label: formatInterface(iface, isContainerSubnetFn),
					color: entities.getColorHelper('Interface').color,
					entityRef: entityRef('Interface', iface.id, iface, { subnets: subnetsData })
				})),
				emptyText: common_notAssigned()
			},
			{ label: common_tags(), snippet: tagsSnippet }
		],
		actions: [
			...(onDelete
				? [
						{
							label: common_delete(),
							icon: Trash2,
							class: 'btn-icon-danger',
							onClick: () => onDelete(service)
						}
					]
				: []),
			...(onEdit
				? [
						{
							label: common_edit(),
							icon: Edit,
							class: 'btn-icon',
							onClick: () => onEdit(service)
						}
					]
				: [])
		]
	});
</script>

{#snippet tagsSnippet()}
	<div class="flex items-center gap-2">
		<span class="text-secondary text-sm">Tags:</span>
		<TagPickerInline selectedTagIds={service.tags} entityId={service.id} entityType="Service" />
	</div>
{/snippet}

<GenericCard {...cardData} {viewMode} {selected} {onSelectionChange} />
