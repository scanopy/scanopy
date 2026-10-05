/**
 * Unified entity UI configuration module.
 * Single source of truth mapping EntityDiscriminants to tab IDs, modal names, and display components.
 */

import type { EntityDiscriminants } from '$lib/api/entities';
import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
import { IPAddressDisplay } from '$lib/shared/components/forms/selection/display/IPAddressDisplay.svelte';
import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
import { DaemonDisplay } from '$lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
import { DependencyDisplay } from '$lib/shared/components/forms/selection/display/DependencyDisplay.svelte';
import { SiteDisplay } from '$lib/shared/components/forms/selection/display/SiteDisplay.svelte';
import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';
import { DaemonApiKeyDisplay } from '$lib/shared/components/forms/selection/display/DaemonApiKeyDisplay.svelte';
import { BindingDisplay } from '$lib/shared/components/forms/selection/display/BindingDisplay.svelte';
import { UserDisplay } from '$lib/shared/components/forms/selection/display/UserDisplay.svelte';
import { DiscoveryDisplay } from '$lib/shared/components/forms/selection/display/DiscoveryDisplay.svelte';
import { PortDisplay } from '$lib/shared/components/forms/selection/display/PortDisplay.svelte';
import { VlanDisplay } from '$lib/shared/components/forms/selection/display/VlanDisplay.svelte';

export interface EntityUIConfig {
	tabId: string;
	modalName?: string;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	displayComponent?: EntityDisplayComponent<any, any>;
	/** For sub-entities: the parent entity type that owns the edit modal */
	parentType?: EntityDiscriminants;
	/** For sub-entities: field name in entity data containing the parent's ID */
	parentIdField?: string;
	/** For sub-entities: which tab to open in the parent's modal */
	modalTab?: string;
	/**
	 * Where one entity of this type opens instead, when it depends on the entity's data: a
	 * discovery run opens in the history tab, a scan configuration in the scans tab.
	 */
	forEntity?: (data: Record<string, unknown>) => EntityUIConfig | undefined;
}

/** Tab ID → display label. Single source of truth for sidebar and back navigation. */
export const TAB_LABELS: Record<string, string> = {
	home: 'Home',
	topology: 'Topology',
	shares: 'Sharing',
	discovery: 'Scans',
	'discovery-scans': 'Scans',
	'discovery-history': 'Historical',
	daemons: 'Daemons',
	'daemon-api-keys': 'API Keys',
	sites: 'Sites',
	subnets: 'Subnets',
	vlans: 'VLANs',
	hosts: 'Hosts',
	services: 'Services',
	tags: 'Tags',
	users: 'Users',
	'api-keys': 'API Keys',
	credentials: 'Credentials'
};

export const entityUIConfig: Record<EntityDiscriminants, EntityUIConfig | null> = {
	Host: { tabId: 'hosts', modalName: 'host-editor', displayComponent: HostDisplay },
	Service: { tabId: 'services', modalName: 'service-editor', displayComponent: ServiceDisplay },
	IPAddress: {
		tabId: 'hosts',
		displayComponent: IPAddressDisplay,
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'ip-addresses'
	},
	Interface: {
		tabId: 'hosts',
		displayComponent: InterfaceDisplay,
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'interfaces'
	},
	// View-only tab: no edit modal, so no `modalName`. Navigating to a VLAN opens its tab.
	Vlan: { tabId: 'vlans', displayComponent: VlanDisplay },
	Port: {
		tabId: 'hosts',
		displayComponent: PortDisplay,
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'ports'
	},
	Binding: {
		tabId: 'hosts',
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'services',
		displayComponent: BindingDisplay
	},
	Subnet: { tabId: 'subnets', modalName: 'subnet-editor', displayComponent: SubnetDisplay },
	Daemon: { tabId: 'daemons', modalName: 'daemon-editor', displayComponent: DaemonDisplay },
	DaemonApiKey: {
		tabId: 'daemon-api-keys',
		modalName: 'daemon-api-key',
		displayComponent: DaemonApiKeyDisplay
	},
	Dependency: {
		tabId: 'dependencies',
		modalName: 'dependency-editor',
		displayComponent: DependencyDisplay
	},
	Site: { tabId: 'sites', modalName: 'site-editor', displayComponent: SiteDisplay },
	Credential: {
		tabId: 'credentials',
		modalName: 'credential-editor',
		displayComponent: CredentialDisplay
	},
	Discovery: {
		tabId: 'discovery-scans',
		modalName: 'discovery-editor',
		displayComponent: DiscoveryDisplay,
		forEntity: (data) =>
			(data.run_type as { type?: string } | undefined)?.type === 'Historical'
				? { tabId: 'discovery-history', modalName: 'discovery-history-detail' }
				: undefined
	},
	Tag: { tabId: 'tags', modalName: 'tag-editor' },
	Share: { tabId: 'shares', modalName: 'share-editor' },
	Topology: { tabId: 'topology', modalName: 'topology-editor', displayComponent: TopologyDisplay },
	Snapshot: { tabId: 'topology' },
	User: { tabId: 'users', modalName: 'user-editor', displayComponent: UserDisplay },
	UserApiKey: { tabId: 'api-keys', modalName: 'user-api-key' },
	Organization: null,
	Invite: null,
	Unknown: null
};
