/**
 * Unified entity UI configuration module.
 * Single source of truth mapping EntityDiscriminants to tab IDs, modal names, and display components.
 */

import type { EntityDiscriminants } from '$lib/api/entities';
import type { EntityDisplayComponent } from '$lib/shared/components/forms/selection/types';
import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
import { ServiceDisplay } from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
import { InterfaceDisplay } from '$lib/shared/components/forms/selection/display/InterfaceDisplay.svelte';
import { IfEntryDisplay } from '$lib/shared/components/forms/selection/display/IfEntryDisplay.svelte';
import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
import { DaemonDisplay } from '$lib/shared/components/forms/selection/display/DaemonDisplay.svelte';
import { GroupDisplay } from '$lib/shared/components/forms/selection/display/GroupDisplay.svelte';
import { NetworkDisplay } from '$lib/shared/components/forms/selection/display/NetworkDisplay.svelte';
import { CredentialDisplay } from '$lib/shared/components/forms/selection/display/CredentialDisplay.svelte';
import { TopologyDisplay } from '$lib/shared/components/forms/selection/display/TopologyDisplay.svelte';
import { DaemonApiKeyDisplay } from '$lib/shared/components/forms/selection/display/DaemonApiKeyDisplay.svelte';

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
}

/** Tab ID → display label. Single source of truth for sidebar and back navigation. */
export const TAB_LABELS: Record<string, string> = {
	home: 'Home',
	topology: 'Topology',
	groups: 'Groups',
	shares: 'Sharing',
	discovery: 'Scans',
	'discovery-sessions': 'Sessions',
	'discovery-scheduled': 'Scheduled',
	'discovery-history': 'History',
	daemons: 'Daemons',
	'daemon-api-keys': 'API Keys',
	networks: 'Networks',
	subnets: 'Subnets',
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
	Interface: {
		tabId: 'hosts',
		displayComponent: InterfaceDisplay,
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'interfaces'
	},
	IfEntry: {
		tabId: 'hosts',
		displayComponent: IfEntryDisplay,
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'if-entries'
	},
	Port: { tabId: 'hosts', parentType: 'Host', parentIdField: 'host_id', modalTab: 'ports' },
	Binding: {
		tabId: 'hosts',
		parentType: 'Host',
		parentIdField: 'host_id',
		modalTab: 'services'
	},
	Subnet: { tabId: 'subnets', modalName: 'subnet-editor', displayComponent: SubnetDisplay },
	Daemon: { tabId: 'daemons', displayComponent: DaemonDisplay },
	DaemonApiKey: {
		tabId: 'daemon-api-keys',
		modalName: 'daemon-api-key',
		displayComponent: DaemonApiKeyDisplay
	},
	Group: { tabId: 'groups', modalName: 'group-editor', displayComponent: GroupDisplay },
	Network: { tabId: 'networks', modalName: 'network-editor', displayComponent: NetworkDisplay },
	Credential: {
		tabId: 'credentials',
		modalName: 'credential-editor',
		displayComponent: CredentialDisplay
	},
	Discovery: { tabId: 'discovery-scheduled', modalName: 'discovery-editor' },
	Tag: { tabId: 'tags', modalName: 'tag-editor' },
	Share: { tabId: 'shares', modalName: 'share-editor' },
	Topology: { tabId: 'topology', modalName: 'topology-editor', displayComponent: TopologyDisplay },
	User: { tabId: 'users', modalName: 'user-editor' },
	UserApiKey: { tabId: 'api-keys', modalName: 'user-api-key' },
	Organization: null,
	Invite: null,
	Unknown: null
};
