<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { Node } from '@xyflow/svelte';
	import type { components } from '#lib/api/schema';
	import type { RenderableTopology, TopologyNode } from '#lib/features/topology/types/base.ts';
	import { nodeEntity, topologyLookup } from '#lib/features/topology/resolvers.ts';
	import {
		entityDisplayText,
		entityFieldLabel,
		entityFields,
		fieldEntityType,
		fieldSource,
		formatFieldValue,
		resolveField,
		type EntityField
	} from '#lib/shared/entity-fields/index.ts';
	import { entityRef } from '#lib/shared/components/data/types.ts';
	import { entities, inspectorSections } from '#lib/shared/stores/metadata.ts';
	import InfoRow from '#lib/shared/components/data/InfoRow.svelte';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import CopyableValue from '#lib/shared/components/data/CopyableValue.svelte';
	import OsTag from '#lib/features/hosts/components/OsTag.svelte';
	import type { HostOs } from '#lib/features/hosts/host-os.ts';
	import type { AttributeSource } from '#lib/shared/utils/attribute-source.ts';
	import type { InspectorSection } from '../view-config';

	type Entity = components['schemas']['EntityDiscriminants'];

	/**
	 * The selected shape's entity, described by fields from the backend field registry
	 * (`shared/entity-fields`), under its card in the Identity section. A field is left out when
	 * the card already draws it, or when it points at an entity another section of this panel
	 * shows (`shown_entity` in the section metadata).
	 */
	let {
		node,
		topology,
		sections,
		allFields = false,
		children
	}: {
		node: Node;
		topology: RenderableTopology;
		/** The panel's section list for this node. */
		sections: InspectorSection[];
		/** Every single-valued field, not only the registry's summary fields. */
		allFields?: boolean;
		/** Rows the registry cannot express, drawn after the field rows. */
		children?: Snippet;
	} = $props();

	/** Fields each entity's Identity card already draws. */
	const DRAWN_BY_CARD: Partial<Record<Entity, ReadonlySet<string>>> = {
		Host: new Set(['name']),
		Service: new Set(['name']),
		Interface: new Set(['display_name', 'mac_address', 'oper_status', 'ip_address_id']),
		IPAddress: new Set(['ip_address', 'mac_address']),
		Subnet: new Set(['name', 'cidr'])
	};

	type Row =
		| {
				field: EntityField;
				label: string;
				type: 'entities';
				items: { id: string; label: string; data: object }[];
		  }
		| { field: EntityField; label: string; type: 'identifiers'; values: string[] }
		| {
				field: EntityField;
				label: string;
				type: 'os';
				os: HostOs;
				source: AttributeSource | null | undefined;
		  }
		| { field: EntityField; label: string; type: 'text'; text: string };

	let resolved = $derived(nodeEntity(node.data as TopologyNode, topology));
	let lookup = $derived(topologyLookup(topology));
	let shownElsewhere = $derived(
		new Set(
			sections
				.filter((section) => section !== 'Identity')
				.map((section) => inspectorSections.getMetadata(section)?.shown_entity)
				.filter((entity): entity is Entity => !!entity)
		)
	);

	function included(entityType: Entity, field: EntityField): boolean {
		// A list of related entities (a VLAN's interfaces) is shown here, where it has room; the
		// hover tooltip leaves it out.
		const relatedList = field.many && field.format.type === 'Entity';
		if (!relatedList && !(allFields ? !field.many : field.summary)) return false;
		if (DRAWN_BY_CARD[entityType]?.has(field.id)) return false;
		const target = fieldEntityType(field);
		return !(target && shownElsewhere.has(target));
	}

	let rows = $derived.by((): Row[] => {
		if (!resolved) return [];
		const { entityType, entity } = resolved;
		const out: Row[] = [];
		for (const field of entityFields(entityType).filter((f) => included(entityType, f))) {
			const values = resolveField(field, entity, lookup);
			if (values.length === 0) continue;
			const label = entityFieldLabel(entityType, field);
			const target = fieldEntityType(field);
			if (field.format.type === 'Entity' && target) {
				const items = values.flatMap((value) => {
					const related = value as { id?: string };
					return related?.id
						? [{ id: related.id, label: entityDisplayText(target, related), data: related }]
						: [];
				});
				if (items.length) out.push({ field, label, type: 'entities', items });
			} else if (field.format.type === 'OperatingSystem' && typeof values[0] === 'object') {
				out.push({
					field,
					label,
					type: 'os',
					os: values[0] as HostOs,
					source: fieldSource(field, entity)
				});
			} else if (field.format.type === 'Identifier') {
				out.push({
					field,
					label,
					type: 'identifiers',
					values: values.map((value) => formatFieldValue(field, value)).filter(Boolean)
				});
			} else {
				const text = values
					.map((value) => formatFieldValue(field, value))
					.filter(Boolean)
					.join(', ');
				if (text) out.push({ field, label, type: 'text', text });
			}
		}
		return out;
	});
</script>

{#if rows.length > 0 || children}
	<div class="card-static card space-y-1.5">
		{#each rows as row (row.field.id)}
			<InfoRow label={row.label}>
				<span class="inline-flex flex-wrap items-center justify-end gap-1">
					{#if row.type === 'entities'}
						{@const target = fieldEntityType(row.field)!}
						{#each row.items as item (item.id)}
							<EntityTag
								entityRef={entityRef(target, item.id, item.data)}
								label={item.label}
								color={entities.getColorHelper(target).color}
							/>
						{/each}
					{:else if row.type === 'os'}
						<OsTag os={row.os} source={row.source} />
					{:else if row.type === 'identifiers'}
						{#each row.values as value (value)}
							<CopyableValue {value} variant="inline" />
						{/each}
					{:else}
						<span class="text-sm text-primary">{row.text}</span>
					{/if}
				</span>
			</InfoRow>
		{/each}
		{@render children?.()}
	</div>
{/if}
