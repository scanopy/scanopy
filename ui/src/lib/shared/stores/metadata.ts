import { writable, get } from 'svelte/store';
import type { components } from '$lib/api/schema';
import serviceDefinitionsJson from '$lib/data/service-definitions.json';
import subnetTypesJson from '$lib/data/subnet-types.json';
import edgeTypesJson from '$lib/data/edge-types.json';
import groupTypesJson from '$lib/data/group-types.json';
import entitiesJson from '$lib/data/entities.json';
import portsJson from '$lib/data/ports.json';
import discoveryTypesJson from '$lib/data/discovery-types.json';
import billingPlansJson from '$lib/data/billing-plans-all.json';
import featuresJson from '$lib/data/features.json';
import permissionsJson from '$lib/data/permissions.json';
import credentialTypesJson from '$lib/data/credential-types.json';
import conceptsJson from '$lib/data/concepts.json';
import {
	createColorHelper,
	createIconComponent,
	createLogoIconComponent,
	createStyle,
	type ColorStyle
} from '../utils/styling';

export type Color = components['schemas']['Color'];

// Base metadata types (not in OpenAPI schema — defined from static fixture shape)
export interface EntityMetadata {
	id: string;
	color: Color | null;
	icon: string | null;
}

export interface TypeMetadata extends EntityMetadata {
	name: string | null;
	description: string | null;
	category: string | null;
	metadata: unknown;
}

export interface FieldDefinition {
	id: string;
	label: string;
	field_type: 'string' | 'text' | 'select' | 'secretpathorinline' | 'pathorinline';
	placeholder?: string;
	secret: boolean;
	optional: boolean;
	help_text?: string;
	options?: string[];
	default_value?: string;
	inline_format?: 'plain' | 'pemprivatekey' | 'pemcertificate';
	group?: string;
}

export interface CredentialTypeMetadata {
	fields: FieldDefinition[];
	/** How this credential type can be scoped to targets */
	scope_models?: string[];
	/** Name of the associated ServiceDefinition (e.g. "SNMP", "Docker") */
	associated_service?: string;
	/** Whether the associated service has a logo */
	has_logo?: boolean;
	/** URL to the service logo */
	logo_url?: string;
	/** Whether the logo needs a white background */
	logo_needs_white_background?: boolean;
}

export interface MetadataRegistry {
	service_definitions: TypeMetadata[];
	subnet_types: TypeMetadata[];
	edge_types: TypeMetadata[];
	group_types: TypeMetadata[];
	entities: EntityMetadata[];
	ports: TypeMetadata[];
	discovery_types: TypeMetadata[];
	billing_plans: TypeMetadata[];
	features: TypeMetadata[];
	permissions: TypeMetadata[];
	concepts: EntityMetadata[];
	credential_types: TypeMetadata[];
}

// Utility type to add proper typing to the metadata field
export type TypedTypeMetadata<TMetadata> = Omit<TypeMetadata, 'metadata'> & {
	metadata: TMetadata;
};

export interface BillingPlanFeatures {
	share_views: boolean;
	remove_created_with: boolean;
	audit_logs: boolean;
	webhooks: boolean;
	api_access: boolean;
	onboarding_call: boolean;
	custom_sso: boolean;
	managed_deployment: boolean;
	whitelabeling: boolean;
	live_chat_support: boolean;
	embeds: boolean;
	email_support: boolean;
	community_support: boolean;
	priority_support: boolean;
	scheduled_discovery: boolean;
	daemon_poll: boolean;
	service_definitions: boolean;
	docker_integration: boolean;
	real_time_updates: boolean;
	snmp_integration: boolean;
	png_export: boolean;
	svg_export: boolean;
	mermaid_export: boolean;
	confluence_export: boolean;
}

export type FeatureId = keyof BillingPlanFeatures;

/** Feature IDs plus resource-based upgrade reasons */
export type UpgradeFeature = FeatureId | 'seats' | 'networks' | 'hosts' | 'plan_usage';

export interface BillingPlanMetadata {
	features: BillingPlanFeatures;
	is_commercial: boolean;
	hosting: string;
	custom_price: string | null;
	incremental_features: string[];
	previous_tier: string | null;
}

export interface ServicedDefinitionMetadata {
	can_be_added: boolean;
	manages_virtualization: 'vms' | 'containers';
	has_logo: boolean;
	logo_url: string;
	has_raw_socket_endpoint: boolean;
}

export interface PermissionsMetadata {
	/** Permission levels this user can assign to API keys (own level or below) */
	grantable_api_key_permissions: string[];
	/** Permission levels this user can assign to other users (Owners can grant all, Admins can grant Member/Viewer) */
	grantable_user_permissions: string[];
	manage_org_entities: boolean;
}

export interface SubnetTypeMetadata {
	network_scan_discovery_eligible: boolean;
	is_for_containers: boolean;
	show_label: boolean;
	hide_from_subnet_list: boolean;
}

export interface EdgeTypeMetadata {
	is_dashed: boolean;
	has_start_marker: boolean;
	has_end_marker: boolean;
	edge_style: 'Straight' | 'Smoothstep' | 'Bezier' | 'Simplebezier' | 'Step';
	is_group_edge: boolean;
	is_host_edge: boolean;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface GroupTypeMetadata {}

export interface FeatureMetadata {
	is_coming_soon: boolean;
	minimum_plan: string | null;
}

export interface PortTypeMetadata {
	is_management: boolean;
	is_dns: boolean;
	is_custom: boolean;
	can_be_added: boolean;
	number: number;
	protocol: 'Tcp' | 'Udp';
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
export interface DiscoveryTypeMetadata {}

export const metadata = writable<MetadataRegistry>({
	service_definitions: serviceDefinitionsJson,
	subnet_types: subnetTypesJson,
	edge_types: edgeTypesJson,
	group_types: groupTypesJson,
	entities: entitiesJson,
	ports: portsJson,
	discovery_types: discoveryTypesJson,
	billing_plans: billingPlansJson,
	features: featuresJson,
	permissions: permissionsJson,
	concepts: conceptsJson,
	credential_types: credentialTypesJson
} as unknown as MetadataRegistry);

// Shared color helper functions that work for both TypeMetadata and EntityMetadata
function createSharedHelpers<T extends keyof MetadataRegistry>(category: T) {
	return {
		getColorString: (id: string | null): Color => {
			const $registry = get(metadata);
			const item = $registry?.[category]?.find((i: EntityMetadata) => i.id === id);
			return item?.color || 'Gray';
		},

		getColorHelper: (id: string | null): ColorStyle => {
			const $registry = get(metadata);
			const item = $registry?.[category]?.find((i: EntityMetadata) => i.id === id);
			const baseColor = item?.color || null;
			return createColorHelper(baseColor);
		},

		getIcon: (id: string | null) => {
			const $registry = get(metadata);
			return (
				($registry?.[category] as EntityMetadata[])?.find((item) => item.id === id)?.icon ||
				'HelpCircle'
			);
		},

		getIconComponent: (id: string | null) => {
			const $registry = get(metadata);
			const item = ($registry?.[category] as EntityMetadata[])?.find((item) => item.id === id);
			const iconName = item?.icon || null;
			return createIconComponent(iconName);
		},

		getStyle: (id: string | null) => {
			const $registry = get(metadata);
			const item = ($registry?.[category] as EntityMetadata[])?.find((item) => item.id === id);
			const color = item?.color || null;
			const icon = item?.icon || null;
			return createStyle(color, icon);
		}
	};
}

// Type helpers to constrain generic types
type TypeMetadataKeys = {
	[K in keyof MetadataRegistry]: MetadataRegistry[K][number] extends TypeMetadata ? K : never;
}[keyof MetadataRegistry];

type EntityMetadataKeys = {
	[K in keyof MetadataRegistry]: MetadataRegistry[K][number] extends EntityMetadata ? K : never;
}[keyof MetadataRegistry];

// Full TypeMetadata helpers (includes color methods + other methods)
function createTypeMetadataHelpers<T extends TypeMetadataKeys, M = unknown>(category: T) {
	const sharedHelpers = createSharedHelpers(category);

	const helpers = {
		...sharedHelpers,

		getIconComponent: (id: string | null) => {
			const $registry = get(metadata);
			const item = ($registry?.[category] as TypeMetadata[])?.find((item) => item.id === id);
			const iconName = item?.icon || null;

			const meta = item?.metadata;
			if (
				meta &&
				typeof meta === 'object' &&
				'has_logo' in meta &&
				meta.has_logo &&
				'logo_url' in meta
			) {
				if ('logo_needs_white_background' in meta) {
					return createLogoIconComponent(
						iconName,
						meta.logo_url as string,
						!!meta.logo_needs_white_background
					);
				}
				return createLogoIconComponent(iconName, meta.logo_url as string);
			}

			return createIconComponent(iconName);
		},

		getItems: (): TypedTypeMetadata<M>[] => {
			const $registry = get(metadata);
			return $registry?.[category] as TypedTypeMetadata<M>[];
		},

		getItem: (id: string | null): TypedTypeMetadata<M> | null => {
			const $registry = get(metadata);
			return (
				(($registry?.[category] as TypedTypeMetadata<M>[])?.find((item) => item.id === id) as
					| TypedTypeMetadata<M>
					| undefined) || null
			);
		},

		getName: (id: string | null) => {
			const $registry = get(metadata);
			return (
				($registry?.[category] as TypeMetadata[])?.find((item) => item.id === id)?.name || id || ''
			);
		},

		getDescription: (id: string | null) => {
			const $registry = get(metadata);
			return (
				($registry?.[category] as TypeMetadata[])?.find((item) => item.id === id)?.description || ''
			);
		},

		getCategory: (id: string | null) => {
			const $registry = get(metadata);
			return (
				($registry?.[category] as TypeMetadata[])?.find((item) => item.id === id)?.category || ''
			);
		},

		getMetadata: (id: string | null): M => {
			const $registry = get(metadata);
			return (
				(($registry?.[category] as TypeMetadata[])?.find((item) => item.id === id)?.metadata as
					| M
					| undefined) || ({} as M)
			);
		}
	};

	return helpers;
}

// EntityMetadata helpers (only color methods)
function createEntityMetadataHelpers<T extends EntityMetadataKeys>(category: T) {
	const sharedHelpers = createSharedHelpers(category);

	const helpers = {
		getItems: () => {
			const $registry = get(metadata);
			return $registry?.[category] as EntityMetadata[];
		},

		getItem: (id: string | null) => {
			const $registry = get(metadata);
			return ($registry?.[category] as EntityMetadata[])?.find((item) => item.id === id) || null;
		},

		// Only include the shared color methods
		...sharedHelpers
	};

	return helpers;
}

// Create all the helpers with typed metadata
export const serviceDefinitions = createTypeMetadataHelpers<
	'service_definitions',
	ServicedDefinitionMetadata
>('service_definitions');
export const subnetTypes = createTypeMetadataHelpers<'subnet_types', SubnetTypeMetadata>(
	'subnet_types'
);
export const edgeTypes = createTypeMetadataHelpers<'edge_types', EdgeTypeMetadata>('edge_types');
export const groupTypes = createTypeMetadataHelpers<'group_types', GroupTypeMetadata>(
	'group_types'
);
export const entities = createEntityMetadataHelpers('entities');
export const ports = createTypeMetadataHelpers<'ports', PortTypeMetadata>('ports');
export const discoveryTypes = createTypeMetadataHelpers<'discovery_types', DiscoveryTypeMetadata>(
	'discovery_types'
);
export const billingPlans = createTypeMetadataHelpers<'billing_plans', BillingPlanMetadata>(
	'billing_plans'
);
export const features = createTypeMetadataHelpers<'features', FeatureMetadata>('features');
export const permissions = createTypeMetadataHelpers<'permissions', PermissionsMetadata>(
	'permissions'
);
export const concepts = createEntityMetadataHelpers('concepts');
export const credentialTypes = createTypeMetadataHelpers<
	'credential_types',
	CredentialTypeMetadata
>('credential_types');

/**
 * Generic metadata item structure for static fixtures.
 * Looser than TypeMetadata to allow JSON imports without strict color types.
 */
interface StaticMetadataItem {
	id: string;
	name: string | null;
	description: string | null;
	category: string | null;
	icon: string | null;
	color: string | null;
	metadata: unknown;
}

/**
 * Create metadata helpers from a static metadata array.
 * Used for billing page to avoid runtime API calls by using static JSON fixtures.
 */
export function createStaticHelpers<M>(items: StaticMetadataItem[]) {
	const map = new Map(items.map((i) => [i.id, i]));
	return {
		getMetadata: (id: string | null): M => (map.get(id ?? '')?.metadata as M) ?? ({} as M),
		getName: (id: string | null) => map.get(id ?? '')?.name ?? id ?? '',
		getDescription: (id: string | null) => map.get(id ?? '')?.description ?? '',
		getCategory: (id: string | null) => map.get(id ?? '')?.category ?? '',
		getIconComponent: (id: string | null) => {
			const item = map.get(id ?? '');
			return createIconComponent(item?.icon ?? null);
		},
		getColorHelper: (id: string | null) => {
			const item = map.get(id ?? '');
			// Cast to Color type - static fixtures may have string color values
			return createColorHelper((item?.color as Color) ?? null);
		}
	};
}
