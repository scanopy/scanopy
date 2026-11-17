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
		/** With what the subnet is for, which travels on the `subnet_created` event. */
		onCreate: (data: Subnet, purpose: SubnetCreatePurpose) => Promise<void> | void;
		onUpdate: (id: string, data: Subnet) => Promise<void> | void;
		onClose: () => void;
		onDelete?: ((id: string) => Promise<void> | void) | null;
		name?: string;
	}

	let {
		subnet = null,
		isOpen = false,
		onCreate,
		onUpdate,
		onClose,
		onDelete = null,
		name = undefined
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
	}

	function editing(): Subnet {
		return subnet ?? createEmptySubnetFormData(defaultSiteId);
	}

	function formValues(source: Subnet): SubnetFormValues {
		return {
			// Blank while the subnet is named after its range, which the name field shows.
			name: source.name === source.cidr ? '' : source.name,
			cidr: source.cidr,
			description: source.description ?? '',
			site_id: source.site_id,
			subnet_type: source.subnet_type,
			tags: source.tags,
			vlan_ids: source.vlan_ids ?? [],
			purpose: 'Scan'
		};
	}

	// The create flow's steps. Its first answer decides the rest: a network to scan goes on to how
	// Scanopy reaches it, a network to record goes on to its details. Editing is one page.
	type Step = 'network' | 'reach' | 'details';
	let purpose = $state<SubnetCreatePurpose>('Scan');
	let steps = $derived<Step[]>(purpose === 'Scan' ? ['network', 'reach'] : ['network', 'details']);
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

	// The site and VLANs the VLAN picker reads. Kept as state because the form's own values aren't
	// tracked by `$derived`, and synced to the form for submission.
	let selectedSiteId = $state('');
	let selectedVlanIds = $state<string[]>([]);

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
			const { purpose: chosenPurpose, ...fields } = value;
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
					await onCreate(subnetData, chosenPurpose);
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
		cidrInput = defaults.cidr;
		activeTab = 'network';
		furthestReached = 0;
	}

	// The server says which ranges come from the network and can't be moved by hand.
	let cidrLock = $derived(subnet?.cidr_lock ?? null);

	// For the daemon a locked range names, and for a daemon awaiting its first connection in create.
	const daemonsQuery = useDaemonsQuery({
		enabled: () => !isEditing || cidrLock?.type === 'DaemonInterface'
	});

	// The range typed in the first step, kept as state for the checks below (the form's own value
	// isn't tracked by `$derived`).
	let cidrInput = $state('');
	const subnetsQuery = useSubnetsQuery();
	let checks = $derived(
		!isEditing && cidrInput && !cidrNotation(cidrInput)
			? checkNewCidr(cidrInput.trim(), selectedSiteId, subnetsQuery.data ?? [])
			: null
	);
	let siteOf = (id: string) => sitesData.find((s) => s.id === id);
	let daemonOf = (id: string) => (daemonsQuery.data ?? []).find((d) => d.id === id);
	let subnetChip = (s: SubnetResponse): ChipRef => ({
		type: 'Subnet',
		id: s.id,
		data: s,
		label: s.name
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
	// are offered only to a subnet that already holds one.
	let subnetTypeOptions: SimpleOption[] = $derived(
		subnetTypes
			.getItems()
			.filter(
				(st) => !subnetTypes.getMetadata(st.id).is_seeded_per_site || st.id === subnet?.subnet_type
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
					label: loading ? common_saving() : saveLabel,
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

{#snippet chips(refs: ChipRef[])}
	<div class="mt-1.5 flex flex-wrap items-center gap-1.5">
		{#each refs as ref (ref.id)}
			<EntityTag
				entityRef={entityRef(ref.type, ref.id, ref.data)}
				label={ref.label}
				icon={entities.getIconComponent(ref.type)}
				color={entities.getColorHelper(ref.type).color}
			/>
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
					<form.Field name="site_id">
						{#snippet children()}
							<SelectSite {selectedSiteId} onSiteChange={handleSiteChange} />
						{/snippet}
					</form.Field>

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
							lock?.type === 'DaemonInterface' ? daemonOf(lock.daemon_id) : undefined}
						<InlineWarning
							body={lock?.type === 'DaemonInterface'
								? subnets_existsHereInterfaced({ cidr: existing.cidr })
								: subnets_existsHere({ cidr: existing.cidr })}
						>
							{@render chips([
								...(daemon
									? [{ type: 'Daemon' as const, id: daemon.id, data: daemon, label: daemon.name }]
									: []),
								subnetChip(existing)
							])}
						</InlineWarning>
					{/if}
					{#each checks?.sameElsewhere ?? [] as other (other.id)}
						{@const site = siteOf(other.site_id)}
						<InlineInfo body={subnets_existsElsewhere({ cidr: other.cidr })}>
							{@render chips([
								subnetChip(other),
								...(site
									? [{ type: 'Site' as const, id: site.id, data: site, label: site.name }]
									: [])
							])}
							{#if site}
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
				</div>
			{:else if !isEditing && activeTab === 'reach'}
				<div class="space-y-4">
					{#if pendingDaemon}
						{@const daemon = pendingDaemon}
						<InlineWarning body={subnets_finishDaemon()}>
							{@render chips([{ type: 'Daemon', id: daemon.id, data: daemon, label: daemon.name }])}
						</InlineWarning>
					{:else}
						<p class="text-sm text-secondary">
							{subnets_reachNoDaemon({ cidr: cidrInput.trim() })}
						</p>
					{/if}
				</div>
			{:else}
				<div class="space-y-8">
					<!-- Subnet Details Section -->
					<div class="space-y-4">
						{#if !isEditing}
							<InlineInfo body={subnets_recordNote()} />
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

						<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
							<!-- Site Selection -->
							<form.Field name="site_id">
								{#snippet children(field)}
									<SelectSite {selectedSiteId} onSiteChange={handleSiteChange} />
								{/snippet}
							</form.Field>

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

<DaemonTroubleshootingModal isOpen={troubleshooting} onClose={() => (troubleshooting = false)} />
