/**
 * Resolves the backend's entity field registry (`entities.json` / `edge-types.json` metadata
 * `fields`, declared in `backend/src/server/shared/entity_fields.rs`) against loaded entities.
 *
 * Every surface that describes an entity — list tables, the topology inspector, the diagram
 * exports — reads its fields here, so which facts an entity has, and where a related fact comes
 * from, is decided once on the backend.
 */

import type { components } from '$lib/api/schema';
import { common_no, common_yes } from '$lib/paraglide/messages';
import { entities, edgeTypes, metadataName } from '$lib/shared/stores/metadata';
import { metaFieldLabel } from '$lib/i18n/metadata';
import { formatBitsPerSecond, formatPort, formatTimestamp } from '$lib/shared/utils/formatting';
import { hostOsLabel } from '$lib/features/hosts/host-os';
import type { Port } from '$lib/features/hosts/types/base';
import { attributeSourceLabel, type AttributeSource } from '$lib/shared/utils/attribute-source';
import type { FieldGroup } from '$lib/shared/components/data/types';

export type EntityField = components['schemas']['EntityField'];
export type FieldFormat = components['schemas']['FieldFormat'];
export type Hop = components['schemas']['Hop'];
export type EntityType = components['schemas']['EntityDiscriminants'];
type EdgeType = components['schemas']['EdgeTypeDiscriminants'];
type HostOs = Parameters<typeof hostOsLabel>[0];

type Obj = Record<string, unknown>;

/**
 * Loaded entities of each type, for following relation hops: every entity of `type` whose
 * `column` holds `id` (or holds an array containing it).
 */
export interface EntityLookup {
	byColumn(type: EntityType, column: string, id: string): Obj[];
	/** Whether this lookup holds a collection of `type`, so an empty result means there are none. */
	loads(type: EntityType): boolean;
}

/**
 * A lookup over whatever collections a page has loaded. A type it lacks resolves to nothing, so
 * a relation field on it reads as empty rather than wrong.
 */
export function collectionLookup(
	collections: Partial<Record<EntityType, readonly object[] | undefined>>
): EntityLookup {
	const indexes = new Map<string, Map<string, Obj[]>>();
	return {
		byColumn(type, column, id) {
			const key = `${type}:${column}`;
			let index = indexes.get(key);
			if (!index) {
				index = new Map();
				for (const item of (collections[type] ?? []) as Obj[]) {
					for (const value of idsAt(item, column)) {
						const list = index.get(value);
						if (list) list.push(item);
						else index.set(value, [item]);
					}
				}
				indexes.set(key, index);
			}
			return index.get(id) ?? [];
		},
		loads: (type) => collections[type] !== undefined
	};
}

/** The ids held under `key`: one id, or an array of them. */
function idsAt(object: Obj, key: string): string[] {
	const value = object[key];
	if (Array.isArray(value)) return value.filter((v): v is string => typeof v === 'string');
	return typeof value === 'string' ? [value] : [];
}

/** The values at a dotted path, flattening arrays met along the way. */
function valuesAt(object: Obj, path: string): unknown[] {
	let current: unknown[] = [object];
	for (const segment of path.split('.')) {
		current = current
			.flatMap((v) => (v && typeof v === 'object' ? [(v as Obj)[segment]] : []))
			.flatMap((v) => (Array.isArray(v) ? v : [v]))
			.filter((v) => v !== null && v !== undefined);
	}
	return current;
}

function follow(hop: Hop, objects: Obj[], lookup: EntityLookup): Obj[] {
	switch (hop.type) {
		case 'Embedded':
			return objects.flatMap((o) => valuesAt(o, hop.key) as Obj[]);
		case 'Parent':
			return objects.flatMap((o) =>
				idsAt(o, hop.key).flatMap((id) => lookup.byColumn(hop.entity, 'id', id))
			);
		case 'Children':
			return objects.flatMap((o) =>
				idsAt(o, 'id').flatMap((id) => lookup.byColumn(hop.entity, hop.key, id))
			);
	}
}

/** Every value of `field` on `root`: related objects for an entity field, else raw values. */
export function resolveField(field: EntityField, root: object, lookup: EntityLookup): unknown[] {
	let objects: Obj[] = [root as Obj];
	for (const hop of field.path) objects = follow(hop, objects, lookup);
	return field.field ? objects.flatMap((o) => valuesAt(o, field.field!)) : objects;
}

/** The entity type an `Entity` field's values are: the type its last relation hop reaches. */
export function fieldEntityType(field: EntityField): EntityType | null {
	for (let i = field.path.length - 1; i >= 0; i--) {
		const hop = field.path[i];
		if (hop.type !== 'Embedded') return hop.entity;
	}
	return null;
}

/** An entity's name in text, from its type's display fields. */
export function entityDisplayText(type: EntityType, entity: object): string {
	for (const key of entities.getMetadata(type).display_fields ?? []) {
		const value = (entity as Obj)[key];
		if (value !== null && value !== undefined && value !== '') return String(value);
	}
	return '';
}

/** One value of `field` as text, for outputs that can't link or style it. */
export function formatFieldValue(field: EntityField, value: unknown): string {
	const format = field.format;
	switch (format.type) {
		case 'Entity': {
			const type = fieldEntityType(field);
			return type && value && typeof value === 'object' ? entityDisplayText(type, value) : '';
		}
		case 'Port':
			return value && typeof value === 'object' ? formatPort(value as Port) : String(value);
		case 'OperatingSystem':
			return value && typeof value === 'object' ? hostOsLabel(value as HostOs) : String(value);
		case 'BitsPerSecond':
			return typeof value === 'number' ? formatBitsPerSecond(value) : String(value);
		case 'Boolean':
			return value ? common_yes() : common_no();
		case 'Timestamp':
			return typeof value === 'string' ? formatTimestamp(value) : String(value);
		case 'Metadata':
			return metadataName(format.fixture, String(value));
		case 'AttributeSource':
			return value ? attributeSourceLabel(value as AttributeSource) : '';
		case 'Text':
		case 'Prose':
		case 'Number':
		case 'Identifier':
			return String(value);
	}
}

/** All of a field's values on `root` as one line of text, or `''` when it has none. */
export function fieldText(field: EntityField, root: object, lookup: EntityLookup): string {
	return resolveField(field, root, lookup)
		.map((value) => formatFieldValue(field, value))
		.filter(Boolean)
		.join(', ');
}

/** The registry fields of an entity type. */
export function entityFields(type: EntityType): EntityField[] {
	return entities.getMetadata(type).fields ?? [];
}

/**
 * The column-menu group of a list-table column: its registry field's, else the one the backend
 * declares for the table's own column. `undefined` for a key neither declares.
 */
export function columnGroup(type: EntityType, key: string): FieldGroup | undefined {
	const metadata = entities.getMetadata(type);
	return (
		metadata.fields?.find((field) => field.id === key)?.group ??
		metadata.table_columns?.find((column) => column.id === key)?.group
	);
}

/** The registry fields of an edge type, read from the edge's own data. */
export function edgeFields(type: EdgeType): EntityField[] {
	return edgeTypes.getMetadata(type).fields ?? [];
}

/** A field's translated label. */
export function entityFieldLabel(type: EntityType, field: EntityField): string {
	return metaFieldLabel('entities', type, field.id, field.label);
}

/** An edge-type field's translated label. */
export function edgeFieldLabel(type: EdgeType, field: EntityField): string {
	return metaFieldLabel('edge_types', type, field.id, field.label);
}

/**
 * Where a field's value came from, for an attributed fact: the `source_field` sibling on the
 * entity. `undefined` for fields that record no provenance.
 */
export function fieldSource(
	field: EntityField,
	entity: object
): AttributeSource | null | undefined {
	if (!field.source_field) return undefined;
	return (entity as Obj)[field.source_field] as AttributeSource | null | undefined;
}
