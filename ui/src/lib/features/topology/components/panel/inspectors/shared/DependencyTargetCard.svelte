<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { X } from 'lucide-svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { entities, serviceDefinitions } from '$lib/shared/stores/metadata';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import EntityTagSelect, {
		type EntityTagOption
	} from '$lib/shared/components/forms/selection/EntityTagSelect.svelte';
	import BindingPicker from './BindingPicker.svelte';
	import {
		common_host,
		common_at,
		common_remove,
		dependencies_noServicesError
	} from '$lib/paraglide/messages';
	import type { DependencyTarget } from '../../../../resolvers';
	import type { RenderableTopology } from '../../../../types/base';

	let {
		form,
		topology,
		target,
		flatIndex,
		onRemove
	}: {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: any;
		topology: RenderableTopology;
		target: DependencyTarget;
		flatIndex: number;
		/** Remove this card's target from the dep (and the canvas selection in create mode). */
		onRemove?: () => void;
	} = $props();

	// Mirror form state — form.state.values isn't tracked by Svelte 5 $derived.
	// Seeded from the form's current value; effect below keeps it in sync.
	interface MirroredValues {
		memberMode: 'Services' | 'Bindings';
		picks: Record<string, string>;
	}
	// svelte-ignore state_referenced_locally
	let formValues = $state<MirroredValues>({
		memberMode: form.state.values.memberMode ?? 'Services',
		picks: { ...(form.state.values.picks ?? {}) }
	});
	$effect(() => {
		return form.store.subscribe(() => {
			formValues = {
				memberMode: form.state.values.memberMode ?? 'Services',
				picks: { ...(form.state.values.picks ?? {}) }
			};
		});
	});

	let memberMode = $derived(formValues.memberMode);

	let host = $derived(
		target.type === 'service'
			? topology.hosts.find(
					(h) => h.id === topology.services.find((s) => s.id === target.serviceId)?.host_id
				)
			: topology.hosts.find((h) => h.id === target.hostId)
	);

	let candidates = $derived(
		target.type === 'service'
			? []
			: target.candidateServiceIds
					.map((id) => topology.services.find((s) => s.id === id))
					.filter((s): s is NonNullable<typeof s> => !!s)
	);

	let resolvedServiceId = $derived.by((): string | null => {
		if (target.type === 'service') return target.serviceId;
		if (candidates.length === 0) return null;
		if (candidates.length === 1) return candidates[0].id;
		return formValues.picks[target.elementId] ?? candidates[0].id;
	});

	let resolvedService = $derived(
		resolvedServiceId ? topology.services.find((s) => s.id === resolvedServiceId) : undefined
	);

	let ipAddressIdFilter = $derived(target.type === 'ipAddress' ? target.ipAddressId : null);

	// Seed default pick for multi-candidate targets so a service resolves on first render.
	$effect(() => {
		if (target.type === 'service') return;
		if (candidates.length <= 1) return;
		if (formValues.picks[target.elementId]) return;
		form.setFieldValue(`picks.${target.elementId}`, candidates[0].id);
	});

	function serviceIcon(serviceDef: string | null | undefined) {
		return serviceDef
			? serviceDefinitions.getIconComponent(serviceDef)
			: entities.getIconComponent('Service');
	}

	function serviceIconColor(serviceDef: string | null | undefined) {
		return serviceDef
			? serviceDefinitions.getColorHelper(serviceDef).color
			: entities.getColorHelper('Service').color;
	}

	let hasPicker = $derived(target.type !== 'service' && candidates.length > 1);

	// Host / IPAddress targets with zero candidate services are invalid members —
	// the card surfaces an inline error, and InspectorMultiSelect blocks submit.
	let hasNoServices = $derived(target.type !== 'service' && candidates.length === 0);
	let noServicesLabel = $derived(
		target.type === 'ipAddress' ? target.label : host ? hostDisplayName(host) : ''
	);

	let serviceOptions = $derived<EntityTagOption[]>(
		candidates.map((svc) => ({
			id: svc.id,
			entityRef: entityRef('Service', svc.id, svc),
			label: svc.name,
			icon: serviceIcon(svc.service_definition),
			color: serviceIconColor(svc.service_definition)
		}))
	);
</script>

<!-- Fade + overflow-hidden live on an inner wrapper so the card's border
     stays fully visible. The X button is absolute-positioned on top of the
     content so the content column spans the full card width — otherwise the
     fade starts to the left of the X button and cuts off dropdown chevrons
     that still have room. -->
<div class="card card-static relative p-2 text-sm">
	<!-- pr reserves space for the fade region (and the X button when present) so
	     dropdown chevrons / tag tails never sit inside the fade. -->
	<div class="dep-card-content space-y-2 overflow-hidden {onRemove ? 'pr-7' : 'pr-4'}">
		<!-- Line 1: Host {host} -->
		<div class="flex items-center gap-1.5">
			<span class="text-tertiary flex-shrink-0">{common_host()}</span>
			{#if host}
				<EntityTag
					entityRef={entityRef('Host', host.id, host)}
					label={hostDisplayName(host)}
					icon={entities.getIconComponent('Host')}
					color={entities.getColorHelper('Host').color}
				/>
			{/if}
		</div>

		<!-- Line 2: running {service} (tag or picker), or error when no services -->
		{#if hasNoServices}
			<p class="text-danger text-xs">
				{dependencies_noServicesError({ name: noServicesLabel })}
			</p>
		{:else}
			<div class="flex items-center gap-1.5">
				<span class="text-tertiary flex-shrink-0">running</span>
				{#if hasPicker}
					<form.Field name="picks.{target.elementId}">
						{#snippet children(field: AnyFieldApi)}
							<div class="min-w-0 flex-1">
								<EntityTagSelect
									options={serviceOptions}
									selectedValue={field.state.value ?? resolvedServiceId}
									onSelect={(id) => field.handleChange(id)}
								/>
							</div>
						{/snippet}
					</form.Field>
				{:else if resolvedService}
					<EntityTag
						entityRef={entityRef('Service', resolvedService.id, resolvedService)}
						label={resolvedService.name}
						icon={serviceIcon(resolvedService.service_definition)}
						color={serviceIconColor(resolvedService.service_definition)}
					/>
				{:else}
					<span class="text-tertiary text-xs italic">—</span>
				{/if}
			</div>

			<!-- Line 3 (Bindings mode): at {binding} (tag or picker) -->
			{#if memberMode === 'Bindings' && resolvedServiceId}
				<div class="flex items-center gap-1.5">
					<span class="text-tertiary flex-shrink-0">{common_at()}</span>
					<BindingPicker
						{form}
						{topology}
						serviceId={resolvedServiceId}
						elementId={target.elementId}
						{flatIndex}
						{ipAddressIdFilter}
					/>
				</div>
			{/if}
		{/if}
	</div>

	{#if onRemove}
		<button
			type="button"
			class="btn-icon absolute right-2 top-2 p-1"
			onclick={onRemove}
			title={common_remove()}
			aria-label={common_remove()}
		>
			<X class="h-4 w-4" />
		</button>
	{/if}
</div>

<style>
	.dep-card-content {
		-webkit-mask-image: linear-gradient(to right, black calc(100% - 1rem), transparent 100%);
		mask-image: linear-gradient(to right, black calc(100% - 1rem), transparent 100%);
	}
</style>
