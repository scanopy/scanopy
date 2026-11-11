<script lang="ts" module>
	import { required, max } from '#lib/shared/components/forms/validators.ts';

	/** A service name's rule, for its field here and for the hidden one an unselected service keeps. */
	export const validateServiceName = ({ value }: { value: string }) =>
		required(value) || max(100)(value);
</script>

<script lang="ts">
	import type {
		IPAddressBinding,
		PortBinding,
		Service
	} from '#lib/features/services/types/base.ts';
	import { serviceCategories, serviceDefinitions } from '#lib/shared/stores/metadata.ts';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import ListManager from '#lib/shared/components/forms/selection/ListManager.svelte';
	import { pushWarning } from '#lib/shared/stores/feedback.ts';
	import { v4 as uuidv4 } from 'uuid';
	import { useServicesCacheQuery } from '#lib/features/services/queries.ts';
	import { PortBindingDisplay } from '#lib/shared/components/forms/selection/display/PortBindingDisplay.svelte';
	import { IPAddressBindingDisplay } from '#lib/shared/components/forms/selection/display/IPAddressBindingDisplay.svelte';
	import MatchDetails from './MatchDetails.svelte';
	import InfoCard from '#lib/shared/components/data/InfoCard.svelte';
	import ConfigHeader from '#lib/shared/components/forms/config/ConfigHeader.svelte';
	import type { HostFormData } from '#lib/features/hosts/types/base.ts';
	import TagPickerInline from '#lib/features/tags/components/TagPickerInline.svelte';
	import { usePortsQuery } from '#lib/features/ports/queries.ts';
	import { useSubnetsQuery, isContainerSubnet } from '#lib/features/subnets/queries.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import CopyableValue from '#lib/shared/components/data/CopyableValue.svelte';
	import GuestStateTag from '#lib/shared/components/virtualization/GuestStateTag.svelte';
	import {
		common_tags,
		common_bindings,
		common_status,
		hosts_deviceFacts_composeProject,
		hosts_deviceFacts_containerId,
		hosts_deviceFacts_containerName,
		common_container,
		common_details,
		common_ipAddressBindings,
		common_name,
		common_portBindings,
		hosts_services_bindingsHelp,
		hosts_services_couldNotFindService,
		hosts_services_couldNotFindServiceToRemove,
		hosts_services_ipAddressBindingsHelp,
		hosts_services_namePlaceholder,
		hosts_services_newBinding,
		hosts_services_noAvailableInterfaces,
		hosts_services_noAvailablePortCombos,
		hosts_services_networkIdentitiesHint,
		hosts_services_networkIdentitiesLearnMore,
		hosts_services_noInterfaces,
		hosts_services_noPorts,
		hosts_services_portBindingsHelp,
		hosts_services_selectBinding,
		hosts_services_unclaimedPortsHint,
		hosts_services_unclaimedPortsLearnMore
	} from '#lib/paraglide/messages.js';

	// TanStack Query hooks
	const servicesQuery = useServicesCacheQuery();
	const portsQuery = usePortsQuery();
	const subnetsQuery = useSubnetsQuery();

	let servicesData = $derived(servicesQuery.data ?? []);
	let portsData = $derived(portsQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	// Get services for a specific port
	function getServicesForPort(portId: string): Service[] {
		const port = portsData.find((p) => p.id === portId);
		if (!port) return [];

		return servicesData.filter(
			(s) =>
				s.host_id === port.host_id &&
				s.bindings.some((b) => b.type === 'Port' && (b as PortBinding).port_id === portId)
		);
	}

	interface Props {
		host: HostFormData;
		service: Service;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		onChange?: (updatedService: Service) => void;
		selectedPortBindings?: PortBinding[];
		index?: number;
		currentServices?: Service[];
	}

	let {
		host,
		service,
		form,
		onChange = () => {},
		selectedPortBindings = $bindable([]),
		index = -1,
		currentServices = []
	}: Props = $props();

	let serviceMetadata = $derived(
		service ? serviceDefinitions.getItem(service.service_definition) : null
	);

	let categoryId = $derived(serviceMetadata?.category ?? null);

	// Field name for this service's name in the form array
	let nameFieldName = $derived(`services[${index}].name`);

	// Notify parent of name changes for real-time sync
	function handleNameChange(value: string) {
		onChange({ ...service, name: value });
	}

	// Port Bindings Logic
	let portBindings = $derived(service.bindings.filter((b) => b.type === 'Port') as PortBinding[]);

	// Interface Bindings Logic
	let ipAddressBindings = $derived(
		service.bindings.filter((b) => b.type === 'IPAddress') as IPAddressBinding[]
	);

	// Get interfaces that this service has Port bindings on
	let interfacesWithPortBindingsThisService = $derived(
		new Set(portBindings.map((b) => b.ip_address_id).filter((id): id is string => id !== null))
	);

	// Check if this service has a Port binding on "All Interfaces"
	let hasPortBindingOnAllIPAddresses = $derived(portBindings.some((b) => b.ip_address_id === null));

	// Get interfaces that this service has Interface bindings on
	let interfacesWithIPAddressBindingsThisService = $derived(
		new Set(ipAddressBindings.map((b) => b.ip_address_id))
	);

	// Available port+interface combinations for new Port bindings
	// Only includes saved interfaces and ports (those not yet in global stores)
	let availablePortCombinations = $derived(
		host.ip_addresses.flatMap((iface) => {
			// Can't add Port binding if THIS service has an Interface binding on this interface
			if (interfacesWithIPAddressBindingsThisService.has(iface.id)) {
				return [];
			}

			return host.ports
				.filter((port) => {
					// Check if this specific port+interface combo is already bound by this service
					const alreadyBoundByThisService = portBindings.some(
						(b) => b.port_id === port.id && b.ip_address_id === iface.id
					);
					if (alreadyBoundByThisService) return false;

					// Get services for this port
					const servicesForPort = getServicesForPort(port.id);
					const otherServices = servicesForPort.filter((s) => s.id !== service.id);

					const boundByOtherService = otherServices.some((s) =>
						s.bindings.some(
							(b) =>
								b.type === 'Port' &&
								(b as PortBinding).port_id === port.id &&
								(b.ip_address_id === iface.id || b.ip_address_id === null)
						)
					);
					if (boundByOtherService) return false;

					// Check if this service has bound this port to ALL interfaces (null)
					const boundToAllIPAddresses = portBindings.some(
						(b) => b.port_id === port.id && b.ip_address_id === null
					);
					if (boundToAllIPAddresses) return false;

					return true;
				})
				.map((port) => ({ port, iface }));
		})
	);

	let canCreatePortBinding = $derived(availablePortCombinations.length > 0);

	// Available interfaces for new Interface bindings
	// Only includes saved interfaces (those already in global store)
	let availableIPAddressesForIPAddressBinding = $derived(
		host.ip_addresses.filter((iface) => {
			// Can't add Interface binding if service has Port binding on "All Interfaces"
			if (hasPortBindingOnAllIPAddresses) return false;

			// Can't add Interface binding if this service already has one on this interface
			if (ipAddressBindings.some((b) => b.ip_address_id === iface.id)) {
				return false;
			}

			// Can't add Interface binding if THIS service has Port bindings on this interface
			if (interfacesWithPortBindingsThisService.has(iface.id)) {
				return false;
			}

			return true;
		})
	);

	let canCreateIPAddressBinding = $derived(availableIPAddressesForIPAddressBinding.length > 0);

	// Port Binding Handlers
	function handleCreatePortBinding() {
		if (!service) {
			pushWarning(hosts_services_couldNotFindService());
			return;
		}

		if (host.ip_addresses.length === 0) {
			pushWarning(hosts_services_noInterfaces());
			return;
		}

		if (host.ports.length === 0) {
			pushWarning(hosts_services_noPorts());
			return;
		}

		if (!canCreatePortBinding) {
			pushWarning(hosts_services_noAvailablePortCombos());
			return;
		}

		const firstAvailable = availablePortCombinations[0];

		const binding: PortBinding = {
			type: 'Port',
			id: uuidv4(),
			service_id: service.id,
			site_id: service.site_id,
			port_id: firstAvailable.port.id,
			ip_address_id: firstAvailable.iface.id,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		};

		onChange({
			...service,
			bindings: [...service.bindings, binding]
		});
	}

	function handleRemovePortBinding(index: number) {
		if (!service) {
			pushWarning(hosts_services_couldNotFindServiceToRemove());
			return;
		}

		const portBindingToRemove = portBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === portBindingToRemove.id);
		onChange({
			...service,
			bindings: service.bindings.filter((_, i) => i !== fullIndex)
		});
	}

	function handleUpdatePortBinding(binding: PortBinding, index: number) {
		if (!service) return;

		const portBindingToUpdate = portBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === portBindingToUpdate.id);

		const updatedBindings = [...service.bindings];
		updatedBindings[fullIndex] = {
			...updatedBindings[fullIndex],
			ip_address_id: binding.ip_address_id,
			port_id: binding.port_id
		} as PortBinding;

		onChange({
			...service,
			bindings: updatedBindings
		});
	}

	// Interface Binding Handlers
	function handleCreateIPAddressBinding() {
		if (!service) {
			pushWarning(hosts_services_couldNotFindService());
			return;
		}

		if (host.ip_addresses.length === 0) {
			pushWarning(hosts_services_noInterfaces());
			return;
		}

		if (!canCreateIPAddressBinding) {
			pushWarning(hosts_services_noAvailableInterfaces());
			return;
		}

		const firstAvailable = availableIPAddressesForIPAddressBinding[0];

		const binding: IPAddressBinding = {
			type: 'IPAddress',
			id: uuidv4(),
			service_id: service.id,
			site_id: service.site_id,
			ip_address_id: firstAvailable.id,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		};

		onChange({
			...service,
			bindings: [...service.bindings, binding]
		});
	}

	function handleRemoveIPAddressBinding(index: number) {
		if (!service) {
			pushWarning(hosts_services_couldNotFindServiceToRemove());
			return;
		}

		const ipAddressBindingToRemove = ipAddressBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === ipAddressBindingToRemove.id);

		onChange({
			...service,
			bindings: service.bindings.filter((_, i) => i !== fullIndex)
		});
	}

	function handleUpdateIPAddressBinding(binding: IPAddressBinding, index: number) {
		if (!service) return;

		const ipAddressBindingToUpdate = ipAddressBindings[index];
		const fullIndex = service.bindings.findIndex((b) => b.id === ipAddressBindingToUpdate.id);

		const updatedBindings = [...service.bindings];
		updatedBindings[fullIndex] = {
			...updatedBindings[fullIndex],
			ip_address_id: binding.ip_address_id
		} as IPAddressBinding;

		onChange({
			...service,
			bindings: updatedBindings
		});
	}
</script>

{#if service && serviceMetadata}
	<div class="space-y-6">
		<ConfigHeader title={serviceMetadata.name ?? ''} subtitle={serviceMetadata.description || null}>
			{#snippet aside()}
				{#if categoryId}
					<Tag
						label={serviceCategories.getName(categoryId)}
						color={serviceCategories.getColorString(categoryId)}
						title={serviceCategories.getDescription(categoryId) || ''}
					/>
				{/if}
			{/snippet}
		</ConfigHeader>

		{#if serviceMetadata.category === 'OpenPorts'}
			<DocsHint
				text={hosts_services_unclaimedPortsHint()}
				href="https://scanopy.net/docs/using-scanopy/network-data/#unclaimed-open-ports"
				linkText={hosts_services_unclaimedPortsLearnMore()}
			/>
		{:else if serviceMetadata.category === 'NetworkIdentities'}
			<DocsHint
				text={hosts_services_networkIdentitiesHint()}
				href="https://scanopy.net/docs/using-scanopy/network-data/#network-identities"
				linkText={hosts_services_networkIdentitiesLearnMore()}
			/>
		{/if}

		<InfoCard title={common_details()}>
			<!-- Service Name Field -->
			<form.Field
				name={nameFieldName}
				validators={{
					onBlur: validateServiceName,
					onChange: validateServiceName
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleNameChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={common_name()}
						id="service_name_{service.id}"
						placeholder={hosts_services_namePlaceholder()}
						required={true}
						{field}
					/>
				{/snippet}
			</form.Field>

			<TagPickerInline
				entityType="Service"
				label={common_tags()}
				selectedTagIds={service.tags}
				onChange={(tags) => onChange({ ...service, tags })}
			/>
		</InfoCard>

		{#if service.virtualization_metadata}
			{@const container = service.virtualization_metadata.details}
			<InfoCard title={common_container()}>
				<dl class="space-y-2 text-sm">
					{#each [{ label: hosts_deviceFacts_containerName(), value: container.container_name }, { label: hosts_deviceFacts_composeProject(), value: container.compose_project }] as fact (fact.label)}
						{#if fact.value}
							<div class="flex flex-wrap items-center gap-x-4 gap-y-1">
								<dt class="w-40 shrink-0 text-secondary">{fact.label}</dt>
								<dd class="min-w-0 flex-1 wrap-break-word text-primary">{fact.value}</dd>
							</div>
						{/if}
					{/each}
					{#if container.container_id}
						<div class="flex flex-wrap items-center gap-x-4 gap-y-1">
							<dt class="w-40 shrink-0 text-secondary">{hosts_deviceFacts_containerId()}</dt>
							<dd class="min-w-0 flex-1">
								<CopyableValue value={container.container_id} variant="inline" />
							</dd>
						</div>
					{/if}
					{#if container.state}
						<div class="flex flex-wrap items-center gap-x-4 gap-y-1">
							<dt class="w-40 shrink-0 text-secondary">{common_status()}</dt>
							<dd class="min-w-0 flex-1"><GuestStateTag state={container.state} /></dd>
						</div>
					{/if}
				</dl>
			</InfoCard>
		{/if}

		<InfoCard title={common_bindings()}>
			<p class="text-xs text-muted">
				{hosts_services_bindingsHelp()}
			</p>
			<!-- Port Bindings -->
			{#key `${service.id}`}
				<ListManager
					label={common_portBindings()}
					helpText={hosts_services_portBindingsHelp()}
					placeholder={hosts_services_selectBinding()}
					createNewLabel={hosts_services_newBinding()}
					allowDuplicates={false}
					allowItemEdit={true}
					allowItemRemove={true}
					allowSelection={true}
					allowReorder={false}
					allowCreateNew={true}
					itemClickAction="select"
					allowAddFromOptions={false}
					disableCreateNewButton={!canCreatePortBinding}
					options={[] as PortBinding[]}
					optionDisplayComponent={PortBindingDisplay}
					itemDisplayComponent={PortBindingDisplay}
					items={portBindings}
					getItemContext={() => ({
						service,
						host,
						services: currentServices.length > 0 ? currentServices : servicesData,
						ip_addresses: host.ip_addresses,
						ports: host.ports,
						isContainerSubnet: isContainerSubnetFn
					})}
					onCreateNew={handleCreatePortBinding}
					onRemove={handleRemovePortBinding}
					onEdit={handleUpdatePortBinding}
					onItemUpdate={(binding, index, updates) =>
						handleUpdatePortBinding({ ...binding, ...updates }, index)}
					bind:selectedItems={selectedPortBindings}
				/>
			{/key}

			<!-- Interface Bindings -->
			{#key service.id}
				<ListManager
					label={common_ipAddressBindings()}
					helpText={hosts_services_ipAddressBindingsHelp()}
					placeholder={hosts_services_selectBinding()}
					createNewLabel={hosts_services_newBinding()}
					allowDuplicates={false}
					allowItemEdit={true}
					allowItemRemove={true}
					allowReorder={false}
					allowCreateNew={true}
					allowAddFromOptions={false}
					disableCreateNewButton={!canCreateIPAddressBinding}
					options={[] as IPAddressBinding[]}
					optionDisplayComponent={IPAddressBindingDisplay}
					itemDisplayComponent={IPAddressBindingDisplay}
					items={ipAddressBindings}
					getItemContext={() => ({
						service,
						host,
						services: currentServices.length > 0 ? currentServices : servicesData,
						ip_addresses: host.ip_addresses,
						isContainerSubnet: isContainerSubnetFn
					})}
					onCreateNew={handleCreateIPAddressBinding}
					onRemove={handleRemoveIPAddressBinding}
					onEdit={handleUpdateIPAddressBinding}
					onItemUpdate={(binding, index, updates) =>
						handleUpdateIPAddressBinding({ ...binding, ...updates }, index)}
				/>
			{/key}
		</InfoCard>

		{#if service.source.type === 'DiscoveryWithMatch' && service.source.details}
			<MatchDetails details={service.source.details} />
		{/if}
	</div>
{/if}
