<script lang="ts">
	import { SvelteMap } from 'svelte/reactivity';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '#lib/shared/components/data/types.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import EntityTagSelect, {
		type EntityTagOption
	} from '#lib/shared/components/forms/selection/EntityTagSelect.svelte';
	import {
		BindingDisplay,
		type BindingDisplayContext
	} from '#lib/shared/components/forms/selection/display/BindingDisplay.svelte';
	import { useSubnetsQuery, isContainerSubnet } from '#lib/features/subnets/queries.ts';
	import {
		dependencies_selectPort,
		dependencies_noOpenPortsError,
		topology_multiSelectNoBindings
	} from '#lib/paraglide/messages.js';
	import type { RenderableTopology } from '../../../../types/base';
	import type { Binding } from '#lib/features/services/types/base.ts';
	import type { IPAddress } from '#lib/features/hosts/types/base.ts';

	let {
		form,
		fieldPrefix = 'bindings',
		topology,
		serviceId,
		elementId,
		flatIndex,
		ipAddressIdFilter = null,
		disabled = false
	}: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: any;
		/** Prefix for the form field path — final field is `${fieldPrefix}.${fieldKey}`. */
		fieldPrefix?: string;
		topology: RenderableTopology;
		serviceId: string;
		/** Unique per-card key for the form entry. Defaults to serviceId when absent
		 *  (backwards compat with callers that use BindingPicker for a single service).
		 *  When multiple cards resolve to the same serviceId, distinct elementIds prevent
		 *  them from racing on a shared form field. */
		elementId?: string;
		flatIndex: number;
		ipAddressIdFilter?: string | null;
		disabled?: boolean;
	} = $props();

	let fieldKey = $derived(elementId ?? serviceId);

	const subnetsQuery = useSubnetsQuery();
	let subnetsData = $derived(subnetsQuery.data ?? []);
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	let bindingContext: BindingDisplayContext = $derived({
		ip_addresses: topology.ip_addresses,
		ports: topology.ports,
		isContainerSubnet: isContainerSubnetFn,
		compact: true
	});

	let backing = $derived(topology.services.find((s) => s.id === serviceId));
	let host = $derived(backing ? topology.hosts.find((h) => h.id === backing!.host_id) : undefined);

	// Mirror the bindings map — form.state.values is not tracked by Svelte 5 $derived.
	// Seeded from the form's current value; effect below keeps it in sync.
	// svelte-ignore state_referenced_locally
	let bindingsMap = $state<Record<string, string>>({
		...((form.state.values[fieldPrefix] ?? {}) as Record<string, string>)
	});
	$effect(() => {
		const { unsubscribe } = form.store.subscribe(() => {
			bindingsMap = {
				...((form.state.values[fieldPrefix] ?? {}) as Record<string, string>)
			};
		});
		return unsubscribe;
	});

	// The first service in a RequestPath is the caller — it initiates the request.
	// It isn't *receiving* traffic here, so the port is irrelevant. Offer IP options
	// instead and map the user's IP pick back to a canonical binding ID.
	let isFirstService = $derived(flatIndex === 0);

	interface IpCandidate {
		ipAddress: IPAddress;
		bindingId: string; // canonical binding for this IP (prefer type=IPAddress, else first Port)
	}

	let ipCandidates = $derived.by((): IpCandidate[] => {
		if (!backing) return [];
		// Group bindings by ip_address_id; skip bindings with null ip_address_id (all-IPs)
		// here — they don't identify a specific instance of the caller.
		const byIp = new SvelteMap<string, { ipOnly?: Binding; firstPort?: Binding }>();
		for (const b of backing.bindings) {
			if (!b.ip_address_id) continue;
			if (ipAddressIdFilter != null && b.ip_address_id !== ipAddressIdFilter) continue;
			const entry = byIp.get(b.ip_address_id) ?? {};
			if (b.type === 'IPAddress') {
				entry.ipOnly = b;
			} else if (b.type === 'Port' && !entry.firstPort) {
				entry.firstPort = b;
			}
			byIp.set(b.ip_address_id, entry);
		}
		const out: IpCandidate[] = [];
		for (const [ipId, entry] of byIp) {
			const canonical = entry.ipOnly ?? entry.firstPort;
			if (!canonical) continue;
			const ip = topology.ip_addresses.find((i) => i.id === ipId);
			if (!ip) continue;
			// Dedupe across cards: skip IPs whose canonical binding is already chosen by another card.
			const takenByOther = Object.entries(bindingsMap).some(
				([otherKey, chosenBindingId]) => otherKey !== fieldKey && chosenBindingId === canonical.id
			);
			if (takenByOther) continue;
			out.push({ ipAddress: ip, bindingId: canonical.id });
		}
		return out;
	});

	let bindingCandidates = $derived.by((): Binding[] => {
		if (!backing) return [];
		return backing.bindings.filter((b) => {
			if (b.type === 'IPAddress') return false; // non-first service can't use IP-only
			if (ipAddressIdFilter != null) {
				if (b.ip_address_id !== ipAddressIdFilter && b.ip_address_id !== null) return false;
			}
			for (const [otherKey, chosenId] of Object.entries(bindingsMap)) {
				if (otherKey !== fieldKey && chosenId === b.id) return false;
			}
			return true;
		});
	});

	// Auto-resolve singleton: first-service uses IP singleton, others use binding singleton.
	$effect(() => {
		if (isFirstService) {
			if (ipCandidates.length !== 1) return;
			if (bindingsMap[fieldKey] === ipCandidates[0].bindingId) return;
			form.setFieldValue(`${fieldPrefix}.${fieldKey}`, ipCandidates[0].bindingId);
		} else {
			if (bindingCandidates.length !== 1) return;
			if (bindingsMap[fieldKey] === bindingCandidates[0].id) return;
			form.setFieldValue(`${fieldPrefix}.${fieldKey}`, bindingCandidates[0].id);
		}
	});

	function ipLabel(ip: IPAddress): string {
		const subnet = subnetsData.find((s) => s.id === ip.subnet_id);
		if (subnet && isContainerSubnet(subnet)) {
			return ip.name ?? ip.ip_address;
		}
		return (ip.name ? ip.name + ': ' : '') + ip.ip_address;
	}

	// Currently-selected IP for first-service display — derived from the bindingId stored in the form.
	let selectedIpCandidate = $derived(
		isFirstService ? ipCandidates.find((c) => c.bindingId === bindingsMap[fieldKey]) : undefined
	);
</script>

{#if backing}
	{#if isFirstService}
		{#if ipCandidates.length === 0}
			<span class="text-xs text-tertiary italic">
				{topology_multiSelectNoBindings()}
			</span>
		{:else if ipCandidates.length === 1}
			<EntityTag
				entityRef={entityRef('IPAddress', ipCandidates[0].ipAddress.id, ipCandidates[0].ipAddress)}
				label={ipLabel(ipCandidates[0].ipAddress)}
				icon={entities.getIconComponent('IPAddress')}
				color={entities.getColorHelper('IPAddress').color}
				disableNavigate={true}
				disablePopover={true}
			/>
		{:else}
			{@const ipOptions: EntityTagOption[] = ipCandidates.map((c) => ({
				id: c.bindingId,
				entityRef: entityRef('IPAddress', c.ipAddress.id, c.ipAddress),
				label: ipLabel(c.ipAddress),
				icon: entities.getIconComponent('IPAddress'),
				color: entities.getColorHelper('IPAddress').color
			}))}
			<div class="min-w-0 flex-1">
				<form.Field name="{fieldPrefix}.{fieldKey}">
					{#snippet children(field: AnyFieldApi)}
						<EntityTagSelect
							options={ipOptions}
							selectedValue={field.state.value ?? selectedIpCandidate?.bindingId ?? null}
							onSelect={(bindingId) => field.handleChange(bindingId)}
							{disabled}
						/>
					{/snippet}
				</form.Field>
			</div>
		{/if}
	{:else if bindingCandidates.length === 0}
		{#if backing.bindings.every((b) => b.type === 'IPAddress')}
			<p class="text-xs text-danger">
				{dependencies_noOpenPortsError({
					serviceName: backing.name,
					hostName: host ? hostDisplayName(host) : ''
				})}
			</p>
		{:else}
			<span class="text-xs text-tertiary italic">
				{topology_multiSelectNoBindings()}
			</span>
		{/if}
	{:else if bindingCandidates.length === 1}
		<EntityTag
			entityRef={entityRef('Binding', bindingCandidates[0].id, bindingCandidates[0])}
			label={BindingDisplay.getLabel?.(bindingCandidates[0], bindingContext) ?? ''}
			icon={entities.getIconComponent('Port')}
			color={entities.getColorHelper('Port').color}
			disableNavigate={true}
			disablePopover={true}
		/>
	{:else}
		{@const bindingOptions: EntityTagOption[] = bindingCandidates.map((b) => ({
			id: b.id,
			entityRef: entityRef('Binding', b.id, b),
			label: BindingDisplay.getLabel?.(b, bindingContext) ?? '',
			icon: entities.getIconComponent('Port'),
			color: entities.getColorHelper('Port').color
		}))}
		<div class="min-w-0 flex-1">
			<form.Field name="{fieldPrefix}.{fieldKey}">
				{#snippet children(field: AnyFieldApi)}
					<EntityTagSelect
						options={bindingOptions}
						selectedValue={field.state.value ?? null}
						placeholder={dependencies_selectPort()}
						onSelect={(bindingId) => field.handleChange(bindingId)}
						{disabled}
					/>
				{/snippet}
			</form.Field>
		</div>
	{/if}
{/if}
