<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { submitForm } from '#lib/shared/components/forms/form-context.ts';
	import { required, max, cidrNotation } from '#lib/shared/components/forms/validators.ts';
	import { createEmptySubnetFormData } from '../../queries';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import { entities, subnetCreatePurposes, subnetTypes } from '#lib/shared/stores/metadata.ts';
	import ModalHeaderIcon from '#lib/shared/components/layout/ModalHeaderIcon.svelte';
	import EntityMetadataSection from '#lib/shared/components/forms/EntityMetadataSection.svelte';
	import type { Subnet, SubnetCreatePurpose, SubnetResponse } from '../../types/base';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import RadioGroup from '#lib/shared/components/forms/input/RadioGroup.svelte';
	import DaemonTroubleshootingModal from '#lib/shared/components/layout/DaemonTroubleshootingModal.svelte';
	import type { ModalTab } from '#lib/shared/components/layout/GenericModal.svelte';
	import { validateForm } from '#lib/shared/components/forms/form-context.ts';
	import { checkNewCidr } from '../../create-checks';
	import { useSubnetsQuery } from '../../queries';
	import { awaitingDaemon } from '#lib/features/daemons/utils.ts';
	import InlineInfo from '#lib/shared/components/feedback/InlineInfo.svelte';
	import { useDaemonsQuery } from '#lib/features/daemons/queries.ts';
	import { openModal } from '#lib/shared/stores/modal-registry.ts';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import MessageWithSlots, { slot } from '#lib/shared/components/data/MessageWithSlots.svelte';
	import { entityRef } from '#lib/shared/components/data/types.ts';
	import type { EntityDiscriminants } from '#lib/api/entities.ts';
	import { useServicesByIds } from '#lib/features/services/queries.ts';
	import SubnetIdentity from './SubnetIdentity.svelte';
	import { attributeSourceLabel } from '#lib/shared/utils/attribute-source.ts';
	import ListManager from '#lib/shared/components/forms/selection/ListManager.svelte';
	import { VlanDisplay } from '#lib/shared/components/forms/selection/display/VlanDisplay.svelte';
	import { useVlansQuery } from '#lib/features/vlans/queries.ts';
	import type { Vlan } from '#lib/features/vlans/types/base.ts';
	import { disabledWhen } from '#lib/shared/components/data/types.ts';
	import SelectSite from '#lib/features/sites/components/SelectSite.svelte';
	import TagPickerInline from '#lib/features/tags/components/TagPickerInline.svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '#lib/shared/components/forms/input/TextArea.svelte';
	import RichSelect from '#lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '#lib/shared/components/forms/selection/display/SimpleOptionDisplay.ts';
	import { useSitesQuery } from '#lib/features/sites/queries.ts';
	import type { ModalAction } from '#lib/shared/components/layout/modalActions.ts';
	import {
		useDiscoveriesQuery,
		useUpdateDiscoveryMutation
	} from '#lib/features/discovery/queries.ts';
	import type { Discovery } from '#lib/features/discovery/types/base.ts';
	import {
		liveUnifiedScans,
		scansTargeting,
		withRoutedTarget
	} from '#lib/features/discovery/utils/targets.ts';
	import { pushError, pushSuccess } from '#lib/shared/stores/feedback.ts';
	import {
		subnets_addToScan,
		subnets_addToThisScan,
		subnets_addedToScan,
		subnets_createAndScan,
		subnets_existsHereScanned,
		subnets_rangeMoveScans,
		subnets_reachInstall,
		subnets_reachInstallHelp,
		subnets_reachQuestion,
		subnets_reachRouted,
		subnets_reachRoutedHelp,
		subnets_routedAddFailed,
		subnets_routedNote,
		subnets_scanLabel,
		subnets_selectScan,
		common_scan,
		common_tags,
		common_back,
		common_cancel,
		common_cidr,
		common_create,
		common_delete,
		common_deleting,
		common_details,
		common_description,
		common_editName,
		common_next,
		common_network,
		common_reach,
		common_saving,
		common_update,
		common_vlans,
		subnets_cidrPlaceholder,
		subnets_containsSubnets,
		subnets_existsElsewhere,
		common_unknown,
		subnets_existsHere,
		subnets_existsHereInterfaced,
		subnets_finishDaemon,
		subnets_mergesAssumed,
		subnets_purposeQuestion,
		subnets_reachNoDaemon,
		subnets_recordNote,
		gettingStarted_stepDaemonLabel,
		home_setup_troubleshootConnection,
		subnets_createSubnet,
		subnets_cidrLockContainerBridge,
		subnets_cidrLockDaemonInterface,
		subnets_cidrLockDiscovered,
		subnets_cidrLockOrganizational,
		subnets_descriptionPlaceholder,
		subnets_noVlans,
		subnets_selectVlan,
		subnets_subnetType,
		subnets_switchToSite,
		subnets_vlanDiscovered
	} from '#lib/paraglide/messages.js';

	// TanStack Query hooks
	const sitesQuery = useSitesQuery();
	let sitesData = $derived(sitesQuery.data ?? []);
	let defaultSiteId = $derived(sitesData[0]?.id ?? '');

	interface Props {
		subnet?: SubnetResponse | null;
		isOpen?: boolean;
		/** With what the subnet is for, which travels on the `subnet_created` event. Returns the
		 *  created subnet, which a subnet created to scan from another daemon is added to its scan
		 *  as. */
		onCreate: (
			data: Subnet,
			purpose: SubnetCreatePurpose
		) => Promise<Subnet | void> | Subnet | void;
		onUpdate: (id: string, data: Subnet) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
		/** Creating a network for one scan, on its site, as when opened from the scan's Targets.
		 *  The flow asks no purpose and no reach, and the caller adds the subnet to the scan. */
		fixedScan?: { siteId: string; label: string } | null;
		/** With `fixedScan`, a subnet that already holds the typed range, to add to the scan in
		 *  place of creating another. */
		onUseExisting?: ((subnet: SubnetResponse) => void) | null;
		/** A range to start from, as when a connected daemon's host isn't on the one asked for. */
		initialCidr?: string;
	}

	let {
		subnet = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined,
		fixedScan = null,
		onUseExisting = null,
		initialCidr = ''
	}: Props = $props();

	let loading = $state(false);
	let deleting = $state(false);

	let isEditing = $derived(subnet !== null);
	let title = $derived(
		isEditing ? common_editName({ name: subnet?.name ?? '' }) : subnets_createSubnet()
	);
	let saveLabel = $derived(isEditing ? common_update() : common_create());

	// The fields the form edits, merged onto the subnet on submit. Not the whole `Subnet`: the
	// generated type is an intersection of the range, name and type carriers, and TanStack's
	// field-path types exceed TypeScript's instantiation depth walking it. The form edits values,
	// never their sources; the server stamps one a person changed `Manual`.
	interface SubnetFormValues {
		name: string;
		cidr: string;
		description: string;
		site_id: string;
		subnet_type: Subnet['subnet_type'];
		tags: string[];
		vlan_ids: string[];
		/** The create flow's first answer. Sent as a query param, not part of the subnet. */
		purpose: SubnetCreatePurpose;
		/** How a network to scan is reached. Neither this nor `scan_id` is part of the subnet. */
		reach: Reach;
		/** The scan a routed network is added to. */
		scan_id: string;
	}

	type Reach = 'install' | 'routed';

	function editing(): Subnet {
		return subnet ?? createEmptySubnetFormData(fixedScan?.siteId ?? defaultSiteId);
	}

	function formValues(source: Subnet): SubnetFormValues {
		return {
			// Blank while the subnet is named after its range, which the name field shows.
			name: source.name === source.cidr ? '' : source.name,
			cidr: subnet ? source.cidr : initialCidr,
			description: source.description ?? '',
			site_id: source.site_id,
			subnet_type: source.subnet_type,
			tags: source.tags,
			vlan_ids: source.vlan_ids ?? [],
			purpose: 'Scan',
			reach: 'install',
			scan_id: ''
		};
	}

	// The site and VLANs the VLAN picker reads. Kept as state because the form's own values aren't
	// tracked by `$derived`, and synced to the form for submission.
	let selectedSiteId = $state('');
	let selectedVlanIds = $state<string[]>([]);

	// The site's scans, for adding a network to one as a routed network, and the scans that already
	// sweep a subnet. The daemons query names the daemon each scan belongs to.
	const daemonsQuery = useDaemonsQuery();
	const discoveriesQuery = useDiscoveriesQuery();
	const updateDiscoveryMutation = useUpdateDiscoveryMutation();
	let allScans = $derived(liveUnifiedScans(discoveriesQuery.data ?? []));
	let siteScans = $derived(allScans.filter((d) => d.site_id === selectedSiteId));
	function scanLabel(d: Discovery): string {
		const daemon = (daemonsQuery.data ?? []).find((x) => x.id === d.daemon_id);
		return subnets_scanLabel({ daemon: daemon?.name ?? common_unknown(), scan: d.name });
	}
	let scanOptions: SimpleOption[] = $derived(
		siteScans.map((d) => ({ value: d.id, label: scanLabel(d) }))
	);

	// The create flow's steps. Its first answer decides the rest: a network to scan goes on to how
	// Scanopy reaches it, and from another daemon's scan to its details; a network to record goes
	// on to its details. A network for a fixed scan skips both questions. Editing is one page.
	type Step = 'network' | 'reach' | 'details';
	let purpose = $state<SubnetCreatePurpose>('Scan');
	let reach = $state<Reach>('install');
	let scanId = $state('');
	let steps = $derived<Step[]>(
		fixedScan || purpose === 'Record'
			? ['network', 'details']
			: reach === 'routed' && siteScans.length > 0
				? ['network', 'reach', 'details']
				: ['network', 'reach']
	);
	let routedToScan = $derived(
		!fixedScan && purpose === 'Scan' && reach === 'routed'
			? siteScans.find((d) => d.id === scanId)
			: undefined
	);
	let activeTab = $state<Step>('network');
	let furthestReached = $state(0);
	let stepIndex = $derived(steps.indexOf(activeTab));
	let tabs: ModalTab[] = $derived(
		isEditing
			? []
			: steps.map((step, i) => ({
					id: step,
					label:
						step === 'network'
							? common_network()
							: step === 'reach'
								? common_reach()
								: common_details(),
					disabled: i > furthestReached
				}))
	);

	let purposeOptions = $derived(
		subnetCreatePurposes.getItems().map((p) => ({
			value: p.id as SubnetCreatePurpose,
			label: subnetCreatePurposes.getName(p.id),
			helpText: subnetCreatePurposes.getDescription(p.id)
		}))
	);

	let reachOptions: { value: Reach; label: string; helpText: string }[] = [
		{ value: 'install', label: subnets_reachInstall(), helpText: subnets_reachInstallHelp() },
		{ value: 'routed', label: subnets_reachRouted(), helpText: subnets_reachRoutedHelp() }
	];

	function handleInstallDaemon() {
		const cidr = cidrInput.trim();
		onClose();
		window.location.hash = 'daemons';
		openModal('create-daemon', { prefill: { sites: selectedSiteId, cidr } });
	}

	async function handleNext() {
		const valid = await validateForm(form);
		if (!valid || checks?.sameHere) return;
		furthestReached = Math.max(furthestReached, stepIndex + 1);
		activeTab = steps[stepIndex + 1];
	}

	function handleBack() {
		if (stepIndex > 0) activeTab = steps[stepIndex - 1];
	}

	function handlePurposeChange(value: SubnetCreatePurpose) {
		purpose = value;
		// The steps after the first change with the answer, so none of them counts as visited.
		furthestReached = 0;
	}

	function handleReachChange(value: Reach) {
		reach = value;
		form.setFieldValue('reach', value);
		furthestReached = Math.min(furthestReached, 1);
	}

	function handleScanChange(id: string) {
		scanId = id;
		form.setFieldValue('scan_id', id);
	}

	/** Add a subnet that already exists to the scan picked for it, and close. */
	async function handleAddExistingToScan(existing: SubnetResponse) {
		if (fixedScan) {
			onUseExisting?.(existing);
			return;
		}
		const scan = siteScans.find((d) => d.id === scanId);
		if (!scan) return;
		try {
			await updateDiscoveryMutation.mutateAsync(withRoutedTarget(scan, existing.id));
			pushSuccess(subnets_addedToScan({ subnet: existing.name, scan: scanLabel(scan) }));
			onClose();
		} catch {
			// The API client shows the error.
		}
	}

	const vlansQuery = useVlansQuery();
	let siteVlans = $derived((vlansQuery.data ?? []).filter((v) => v.site_id === selectedSiteId));
	let vlanOptions = $derived(siteVlans.filter((v) => !selectedVlanIds.includes(v.id)));
	let selectedVlans = $derived(
		selectedVlanIds
			.map((id) => (vlansQuery.data ?? []).find((v) => v.id === id))
			.filter((v): v is Vlan => v !== undefined)
	);
	let discoveredVlanIds = $derived(subnet?.discovered_vlan_ids ?? []);

	function setVlanIds(ids: string[]) {
		selectedVlanIds = ids;
		form.setFieldValue('vlan_ids', ids);
	}

	function handleSiteChange(id: string) {
		selectedSiteId = id;
		form.setFieldValue('site_id', id);
		// A VLAN belongs to one site, so links to the old site's VLANs don't carry over.
		setVlanIds(selectedVlanIds.filter((v) => siteVlans.some((s) => s.id === v)));
	}

	function getDefaultValues(): SubnetFormValues {
		return formValues(editing());
	}

	// Create form with initial empty values - we'll reset it when the modal opens
	const form = createForm(() => ({
		defaultValues: formValues(createEmptySubnetFormData('')),
		onSubmit: async ({ value }) => {
			const { purpose: chosenPurpose, reach: _reach, scan_id: _scanId, ...fields } = value;
			// Read before `onCreate`, which closes the modal.
			const scan = routedToScan;
			const subnetData: Subnet = {
				...editing(),
				...fields,
				// No name names the subnet after its range.
				name: value.name.trim() || value.cidr.trim(),
				description: value.description.trim(),
				cidr: value.cidr.trim()
			};

			loading = true;
			try {
				if (isEditing && subnet) {
					await onUpdate(subnet.id, subnetData);
				} else {
					const created = await onCreate(subnetData, chosenPurpose);
					if (created && scan) {
						try {
							await updateDiscoveryMutation.mutateAsync(withRoutedTarget(scan, created.id));
						} catch {
							pushError(subnets_routedAddFailed({ subnet: created.name, scan: scanLabel(scan) }));
						}
					}
				}
			} finally {
				loading = false;
			}
		}
	}));

	// Reset form when modal opens
	function handleOpen() {
		const defaults = getDefaultValues();
		form.reset(defaults);
		selectedSiteId = defaults.site_id;
		selectedVlanIds = defaults.vlan_ids;
		purpose = defaults.purpose;
		reach = defaults.reach;
		scanId = defaults.scan_id;
		cidrInput = defaults.cidr;
		activeTab = 'network';
		furthestReached = 0;
	}

	// The server says which ranges come from the network and can't be moved by hand.
	let cidrLock = $derived(subnet?.cidr_lock ?? null);

	// The range typed in the first step, kept as state for the checks below (the form's own value
	// isn't tracked by `$derived`).
	let cidrInput = $state('');
	const subnetsQuery = useSubnetsQuery();
	let checks = $derived(
		!isEditing && cidrInput && !cidrNotation(cidrInput)
			? checkNewCidr(cidrInput.trim(), selectedSiteId, subnetsQuery.data ?? [])
			: null
	);
	// The scans that already sweep an existing subnet with the typed range.
	let existingScans = $derived(
		checks?.sameHere ? scansTargeting(checks.sameHere.id, allScans) : []
	);
	// Editing a range a scan sweeps as routed moves what that scan sweeps.
	let movedRangeScans = $derived(
		isEditing && subnet && cidrInput.trim() && cidrInput.trim() !== subnet.cidr
			? scansTargeting(subnet.id, allScans)
			: []
	);
	let siteOf = (id: string) => sitesData.find((s) => s.id === id);
	let daemonOf = (id: string) => (daemonsQuery.data ?? []).find((d) => d.id === id);
	let subnetChip = (s: SubnetResponse): ChipRef => ({
		type: 'Subnet',
		id: s.id,
		data: s,
		label: s.name
	});
	let siteChip = (id: string): ChipRef | undefined => {
		const site = siteOf(id);
		return site && { type: 'Site', id: site.id, data: site, label: site.name };
	};
	let daemonChip = (id: string): ChipRef | undefined => {
		const daemon = daemonOf(id);
		return daemon && { type: 'Daemon', id: daemon.id, data: daemon, label: daemon.name };
	};
	let scanChip = (d: Discovery): ChipRef => ({
		type: 'Discovery',
		id: d.id,
		data: d,
		label: scanLabel(d)
	});

	// A daemon on this site that was created but never connected is the one to fix before
	// installing another.
	let pendingDaemon = $derived(awaitingDaemon(daemonsQuery.data ?? [], selectedSiteId));
	let troubleshooting = $state(false);
	// The runtime a container bridge's range comes from, for its chip.
	const bridgeServiceQuery = useServicesByIds(() =>
		cidrLock?.type === 'ContainerBridge' ? [cidrLock.service_id] : []
	);

	interface ChipRef {
		type: EntityDiscriminants;
		id: string;
		data: object;
		label: string;
	}

	/** Where the range comes from and when it changes, and the record it comes from, if any. */
	let cidrLockReason = $derived.by((): { body: string; source?: ChipRef } | null => {
		if (!cidrLock || !subnet) return null;
		switch (cidrLock.type) {
			case 'DaemonInterface': {
				const daemon = daemonOf(cidrLock.daemon_id);
				return {
					body: subnets_cidrLockDaemonInterface(),
					source: daemon
						? { type: 'Daemon', id: daemon.id, data: daemon, label: daemon.name }
						: undefined
				};
			}
			case 'ContainerBridge': {
				const service = bridgeServiceQuery.data?.[0];
				return {
					body: subnets_cidrLockContainerBridge(),
					source: service
						? { type: 'Service', id: service.id, data: service, label: service.name }
						: undefined
				};
			}
			case 'Discovered':
				return {
					body: subnets_cidrLockDiscovered({
						source: attributeSourceLabel(subnet.cidr_source ?? 'Unspecified')
					})
				};
			case 'Organizational':
				return { body: subnets_cidrLockOrganizational() };
		}
	});

	async function handleSubmit() {
		await submitForm(form);
	}

	async function handleDelete() {
		if (onDelete && subnet) {
			deleting = true;
			try {
				await onDelete(subnet.id);
			} finally {
				deleting = false;
			}
		}
	}

	let colorHelper = entities.getColorHelper('Subnet');

	// Prepare subnet type options. Every site already has its Internet and Remote ranges, so those
	// are offered only to a subnet that already holds one. A network created to scan is offered
	// only the types a scan sweeps.
	let creatingToScan = $derived(!isEditing && (!!fixedScan || !!routedToScan));
	let subnetTypeOptions: SimpleOption[] = $derived(
		subnetTypes
			.getItems()
			.filter(
				(st) => !subnetTypes.getMetadata(st.id).is_seeded_per_site || st.id === subnet?.subnet_type
			)
			.filter(
				(st) => !creatingToScan || subnetTypes.getMetadata(st.id).network_scan_discovery_eligible
			)
			.map((st) => ({
				value: st.id,
				label: st.name ?? st.id,
				icon: subnetTypes.getIconComponent(st.id),
				iconColor: subnetTypes.getColorHelper(st.id).icon
			}))
	);

	let actions = $derived<ModalAction[]>([
		...(isEditing && onDelete
			? [
					{
						role: 'danger' as const,
						label: deleting ? common_deleting() : common_delete(),
						onClick: handleDelete,
						disabled: deleting || loading
					}
				]
			: []),
		{ role: 'cancel', label: common_cancel(), onClick: onClose, disabled: loading || deleting },
		...(!isEditing && stepIndex > 0
			? [{ role: 'secondary' as const, label: common_back(), onClick: handleBack }]
			: []),
		...primaryAction()
	]);

	/** The step's way forward: Next, Create, or installing a daemon. While a daemon on the site
	 *  still has to connect, troubleshooting it, not installing another. */
	function primaryAction(): ModalAction[] {
		if (isEditing || activeTab === 'details') {
			return [
				{
					role: 'primary',
					label: loading ? common_saving() : routedToScan ? subnets_createAndScan() : saveLabel,
					onClick: handleSubmit,
					disabled: loading || deleting
				}
			];
		}
		if (activeTab === 'network') {
			return [
				{ role: 'primary', label: common_next(), onClick: handleNext, disabled: !!checks?.sameHere }
			];
		}
		if (reach === 'routed' && siteScans.length > 0) {
			return [
				{ role: 'primary', label: common_next(), onClick: handleNext, disabled: !routedToScan }
			];
		}
		return pendingDaemon
			? [
					{
						role: 'primary',
						label: home_setup_troubleshootConnection(),
						onClick: () => (troubleshooting = true)
					}
				]
			: [
					{ role: 'primary', label: gettingStarted_stepDaemonLabel(), onClick: handleInstallDaemon }
				];
	}
</script>

{#snippet chip(ref: ChipRef)}
	<EntityTag
		entityRef={entityRef(ref.type, ref.id, ref.data)}
		label={ref.label}
		icon={entities.getIconComponent(ref.type)}
		color={entities.getColorHelper(ref.type).color}
	/>
{/snippet}

<!-- A message whose `slot(key)` parameters render as the chips in `refs`. -->
{#snippet sentence(text: string, refs: Record<string, ChipRef | undefined>)}
	<p class="leading-6">
		<MessageWithSlots {text}>
			{#snippet part(key)}
				{@const ref = refs[key]}
				{#if ref}
					{@render chip(ref)}
				{/if}
			{/snippet}
		</MessageWithSlots>
	</p>
{/snippet}

{#snippet chips(refs: ChipRef[])}
	<div class="mt-1.5 flex flex-wrap items-center gap-1.5">
		{#each refs as ref (ref.id)}
			{@render chip(ref)}
		{/each}
	</div>
{/snippet}

<!-- Under the locked CIDR field: why it is locked, and the record the range comes from. -->
{#snippet cidrLockHint()}
	{#if cidrLockReason}
		<div class="text-xs text-tertiary">
			<p>{cidrLockReason.body}</p>
			{#if cidrLockReason.source}
				{@render chips([cidrLockReason.source])}
			{/if}
		</div>
	{/if}
{/snippet}

<GenericModal
	{isOpen}
	{title}
	{name}
	entityId={subnet?.id}
	{form}
	size="xl"
	{onClose}
	onOpen={handleOpen}
	showCloseButton={true}
	{tabs}
	{activeTab}
	tabStyle="stepper"
	onTabChange={(id) => (activeTab = id as Step)}
	{actions}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={entities.getIconComponent('Subnet')} color={colorHelper.color} />
	{/snippet}

	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex min-h-0 flex-1 flex-col"
	>
		<div class="min-h-0 flex-1 overflow-auto p-6">
			{#if !isEditing && activeTab === 'network'}
				<div class="space-y-4">
					{#if !fixedScan}
						<form.Field name="site_id">
							<SelectSite {selectedSiteId} onSiteChange={handleSiteChange} />
						</form.Field>
					{/if}

					<form.Field
						name="cidr"
						validators={{ onBlur: ({ value }) => required(value) || cidrNotation(value) }}
						listeners={{ onChange: ({ value }) => (cidrInput = value) }}
					>
						{#snippet children(field)}
							<TextInput
								label={common_cidr()}
								id="cidr"
								{field}
								placeholder={subnets_cidrPlaceholder()}
								required
							/>
						{/snippet}
					</form.Field>

					{#if checks?.sameHere}
						{@const existing = checks.sameHere}
						{@const lock = existing.cidr_lock}
						{@const daemon =
							lock?.type === 'DaemonInterface' ? daemonChip(lock.daemon_id) : undefined}
						{@const site = siteChip(existing.site_id)}
						{@const refs = { subnet: subnetChip(existing), site, daemon }}
						{@const siteSlot = site ? slot('site') : common_unknown()}
						<InlineWarning title="">
							{#if daemon}
								{@render sentence(
									subnets_existsHereInterfaced({
										subnet: slot('subnet'),
										site: siteSlot,
										daemon: slot('daemon')
									}),
									refs
								)}
							{:else if existingScans.length > 0}
								{@render sentence(
									subnets_existsHereScanned({ subnet: slot('subnet'), site: siteSlot }),
									refs
								)}
								{@render chips(existingScans.map(scanChip))}
							{:else}
								{@render sentence(
									subnets_existsHere({ subnet: slot('subnet'), site: siteSlot }),
									refs
								)}
							{/if}
							{#if !daemon && fixedScan}
								<button
									type="button"
									class="btn-secondary mt-2"
									onclick={() => handleAddExistingToScan(existing)}
								>
									{subnets_addToThisScan()}
								</button>
							{:else if !daemon && existingScans.length === 0 && siteScans.length > 0}
								<div class="mt-2 flex flex-wrap items-end gap-2">
									<form.Field name="scan_id">
										<div class="min-w-64 flex-1">
											<RichSelect
												label={common_scan()}
												placeholder={subnets_selectScan()}
												selectedValue={scanId}
												options={scanOptions}
												displayComponent={SimpleOptionDisplay}
												onSelect={handleScanChange}
											/>
										</div>
									</form.Field>
									<button
										type="button"
										class="btn-secondary"
										disabled={!scanId || updateDiscoveryMutation.isPending}
										onclick={() => handleAddExistingToScan(existing)}
									>
										{subnets_addToScan()}
									</button>
								</div>
							{/if}
						</InlineWarning>
					{/if}
					{#each checks?.sameElsewhere ?? [] as other (other.id)}
						{@const site = siteOf(other.site_id)}
						{@const siteRef = siteChip(other.site_id)}
						<InlineInfo>
							{@render sentence(
								subnets_existsElsewhere({
									subnet: slot('subnet'),
									site: siteRef ? slot('site') : common_unknown()
								}),
								{ subnet: subnetChip(other), site: siteRef }
							)}
							{#if site && !fixedScan}
								<button
									type="button"
									class="btn-secondary mt-2"
									onclick={() => handleSiteChange(site.id)}
								>
									{subnets_switchToSite({ site: site.name })}
								</button>
							{/if}
						</InlineInfo>
					{/each}
					{#if checks && checks.assumedInside.length > 0}
						<InlineInfo body={subnets_mergesAssumed()}>
							{@render chips(checks.assumedInside.map(subnetChip))}
						</InlineInfo>
					{/if}
					{#if checks && checks.inside.length > 0}
						<InlineInfo body={subnets_containsSubnets()}>
							{@render chips(checks.inside.map(subnetChip))}
						</InlineInfo>
					{/if}

					{#if !fixedScan}
						<form.Field name="purpose">
							{#snippet children(field)}
								<RadioGroup
									label={subnets_purposeQuestion()}
									id="purpose"
									{field}
									options={purposeOptions}
									value={purpose}
									onChange={handlePurposeChange}
								/>
							{/snippet}
						</form.Field>
					{/if}
				</div>
			{:else if !isEditing && activeTab === 'reach'}
				<div class="space-y-4">
					{#if siteScans.length > 0}
						<form.Field name="reach">
							{#snippet children(field)}
								<RadioGroup
									label={subnets_reachQuestion({ cidr: cidrInput.trim() })}
									id="reach"
									{field}
									options={reachOptions}
									value={reach}
									onChange={handleReachChange}
								/>
							{/snippet}
						</form.Field>
					{:else if !pendingDaemon}
						<p class="text-sm text-secondary">
							{subnets_reachNoDaemon({ cidr: cidrInput.trim() })}
						</p>
					{/if}
					{#if reach === 'routed' && siteScans.length > 0}
						<form.Field name="scan_id">
							<RichSelect
								label={common_scan()}
								placeholder={subnets_selectScan()}
								selectedValue={scanId}
								options={scanOptions}
								displayComponent={SimpleOptionDisplay}
								onSelect={handleScanChange}
								required
							/>
						</form.Field>
					{:else if pendingDaemon}
						{@const daemon = pendingDaemon}
						<InlineWarning title="" body={subnets_finishDaemon()}>
							{@render chips([{ type: 'Daemon', id: daemon.id, data: daemon, label: daemon.name }])}
						</InlineWarning>
					{/if}
				</div>
			{:else}
				<div class="space-y-8">
					<!-- Subnet Details Section -->
					<div class="space-y-4">
						{#if !isEditing}
							<InlineInfo
								body={fixedScan
									? subnets_routedNote({ scan: fixedScan.label })
									: routedToScan
										? subnets_routedNote({ scan: scanLabel(routedToScan) })
										: subnets_recordNote()}
							/>
						{/if}
						<SubnetIdentity {form} cidr={cidrInput.trim()} initialName={form.state.values.name} />

						<form.Field
							name="cidr"
							validators={{
								onBlur: ({ value }) => required(value) || cidrNotation(value)
							}}
							listeners={{ onChange: ({ value }) => (cidrInput = value) }}
						>
							{#snippet children(field)}
								<TextInput
									label={common_cidr()}
									id="cidr"
									{field}
									placeholder={subnets_cidrPlaceholder()}
									disabled={!isEditing}
									disabledReason={cidrLockReason?.body}
									hint={cidrLockReason ? cidrLockHint : undefined}
									required
								/>
							{/snippet}
						</form.Field>
						{#if movedRangeScans.length > 0}
							<InlineWarning title="" body={subnets_rangeMoveScans()}>
								{@render chips(movedRangeScans.map(scanChip))}
							</InlineWarning>
						{/if}

						<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
							<!-- Site Selection. A network created for a scan stays on the scan's site. -->
							{#if !creatingToScan}
								<form.Field name="site_id">
									<SelectSite {selectedSiteId} onSiteChange={handleSiteChange} />
								</form.Field>
							{/if}

							<!-- Subnet Type -->
							<form.Field name="subnet_type">
								{#snippet children(field)}
									<RichSelect
										label={subnets_subnetType()}
										selectedValue={field.state.value}
										options={subnetTypeOptions}
										displayComponent={SimpleOptionDisplay}
										showSearch={true}
										onSelect={(value) => field.handleChange(value as Subnet['subnet_type'])}
									/>
								{/snippet}
							</form.Field>
						</div>

						<!-- Description Field -->
						<form.Field
							name="description"
							validators={{
								onBlur: ({ value }) => max(500)(value || '')
							}}
						>
							{#snippet children(field)}
								<TextArea
									label={common_description()}
									id="description"
									{field}
									placeholder={subnets_descriptionPlaceholder()}
									rows={3}
								/>
							{/snippet}
						</form.Field>

						<ListManager
							label={common_vlans()}
							placeholder={subnets_selectVlan()}
							emptyMessage={subnets_noVlans()}
							allowReorder={false}
							allowItemEdit={false}
							showSearch={true}
							options={vlanOptions}
							items={selectedVlans}
							optionDisplayComponent={VlanDisplay}
							itemDisplayComponent={VlanDisplay}
							itemRemoveUnavailable={(vlan) =>
								disabledWhen(discoveredVlanIds.includes(vlan.id), subnets_vlanDiscovered())}
							onAdd={(id) => setVlanIds([...selectedVlanIds, id])}
							onRemove={(index) => setVlanIds(selectedVlanIds.filter((_, i) => i !== index))}
						/>

						<!-- Tags -->
						<form.Field name="tags">
							{#snippet children(field)}
								<TagPickerInline
									entityType="Subnet"
									label={common_tags()}
									selectedTagIds={field.state.value || []}
									onChange={(tags) => field.handleChange(tags)}
								/>
							{/snippet}
						</form.Field>
					</div>
				</div>
			{/if}
		</div>

		{#if isEditing && subnet}
			<EntityMetadataSection entities={[subnet]} />
		{/if}
	</form>
</GenericModal>

<DaemonTroubleshootingModal
	daemon={pendingDaemon}
	isOpen={troubleshooting}
	onClose={() => (troubleshooting = false)}
/>
