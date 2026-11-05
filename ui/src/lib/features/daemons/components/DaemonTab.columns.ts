import type { Snippet } from 'svelte';
import type { Daemon } from '$lib/features/daemons/types/base';
import {
	daemonStatusTag,
	getDaemonStatus,
	getDaemonStatusTag,
	osLabel,
	daemonOsIds,
	DAEMON_STATUSES
} from '$lib/features/daemons/utils';
import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import type { EntityLookup } from '$lib/shared/entity-fields';
import { tagNames } from '$lib/features/tags/columns';
import { siteItems } from '$lib/features/sites/columns';
import { entityRef } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import type { Subnet } from '$lib/features/subnets/types/base';
import type { Site } from '$lib/features/sites/types';
import type { Tag } from '$lib/features/tags/types/base';
import type { Host } from '$lib/features/hosts/types/base';
import type { User } from '$lib/features/users/types';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import type { components, operations } from '$lib/api/schema';
import { fieldValueOptions, type FieldValueCount } from '$lib/shared/api/field-values';
import {
	common_host,
	common_maintainer,
	common_site,
	common_tags,
	common_unknownEntity,
	common_unknownSite,
	daemons_config_mode,
	daemons_lastSeen,
	daemons_mode_daemonPoll,
	daemons_mode_daemonPollDescription,
	daemons_mode_serverPoll,
	daemons_mode_serverPollDescription
} from '$lib/paraglide/messages';

type DaemonOrderField = components['schemas']['DaemonOrderField'];
/** The daemon list endpoint's query params, which the filters are sent and linked as. */
type DaemonListParam = keyof NonNullable<operations['list_daemons']['parameters']['query']>;

/** What the daemon columns read from the tab, each as a getter or function so values stay live. */
export interface DaemonColumnsContext {
	lookup: () => EntityLookup;
	sites: () => Site[];
	tags: () => Tag[];
	daemonHosts: () => Host[];
	maintainers: () => User[];
	interfacedSubnets: (daemon: Daemon) => Subnet[];
	/** The subnets the management lists show, to label the filter's options. */
	subnets: () => Subnet[];
	osCell: () => Snippet<[Daemon, string]>;
	siteValues: () => FieldValueCount[] | undefined;
	modeValues: () => FieldValueCount[] | undefined;
	maintainerValues: () => FieldValueCount[] | undefined;
	versionValues: () => FieldValueCount[] | undefined;
}

/**
 * Columns from the entity field registry; this adds how each one searches, filters and sorts.
 * Every DaemonOrderField value must be listed; the server sorts, groups and filters each.
 */
export function daemonColumns(ctx: DaemonColumnsContext) {
	return defineRegistryFields<Daemon, DaemonOrderField, DaemonListParam>(
		'Daemon',
		ctx.lookup,
		{
			// Identity field: grouping by it would render a header per daemon.
			name: {
				searchable: true,
				groupable: false,
				display: { order: 0, primary: true, width: 220 }
			},
			site_id: {
				label: common_site(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				// The sites some daemon is on.
				filterOptions: fieldValueOptions(ctx.siteValues()),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? common_unknownSite(),
				// Displayed as a name, but grouped by id on the server.
				getGroupValue: (item) => item.site_id,
				getValue: (item) =>
					ctx.sites().find((n) => n.id == item.site_id)?.name || common_unknownSite(),
				display: { order: 3, getItems: (item) => siteItems(item.site_id, ctx.sites()) }
			},
			last_seen: {
				label: daemons_lastSeen(),
				groupable: false,
				display: { order: 2, recency: true }
			},
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } },
			// The status the server derives, which also carries version lifecycle (deprecated,
			// unsupported). The home page shows the same tag.
			status: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'statuses',
				filterOptions: DAEMON_STATUSES,
				getFilterValue: (daemon) => getDaemonStatus(daemon),
				filterLabel: (value) => {
					const status = DAEMON_STATUSES.find((s) => s === value);
					return status ? daemonStatusTag(status).label : value;
				},
				getGroupValue: (daemon) => getDaemonStatus(daemon),
				getValue: (daemon) => getDaemonStatusTag(daemon).label,
				display: {
					order: 1,
					getItems: (daemon) => {
						const tag = getDaemonStatusTag(daemon);
						return [
							{
								id: tag.label,
								label: tag.label,
								color: tag.color,
								icon: tag.icon,
								href: tag.href
							}
						];
					}
				}
			},
			os: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'os',
				filterOptions: daemonOsIds,
				filterLabel: (value) => {
					const os = daemonOsIds.find((id) => id === value);
					return os ? osLabel(os) : value;
				},
				getFilterValue: (daemon) => daemon.os ?? null,
				getGroupValue: (daemon) => daemon.os ?? null,
				getValue: (daemon) => (daemon.os ? osLabel(daemon.os) : null),
				display: { order: 4.5, cell: ctx.osCell() }
			},
			mode: {
				label: daemons_config_mode(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'modes',
				// The ways some daemon connects.
				filterOptions: fieldValueOptions(ctx.modeValues()),
				getFilterValue: (daemon) => daemon.mode,
				filterLabel: (mode) =>
					mode === 'server_poll' ? daemons_mode_serverPoll() : daemons_mode_daemonPoll(),
				getGroupValue: (daemon) => daemon.mode,
				getValue: (daemon) =>
					daemon.mode === 'server_poll' ? daemons_mode_serverPoll() : daemons_mode_daemonPoll(),
				display: {
					order: 5,
					getItems: (daemon) => [
						{
							id: daemon.mode,
							label:
								daemon.mode === 'server_poll'
									? daemons_mode_serverPoll()
									: daemons_mode_daemonPoll(),
							title:
								daemon.mode === 'server_poll'
									? daemons_mode_serverPollDescription()
									: daemons_mode_daemonPollDescription()
						}
					]
				}
			},
			maintainer: {
				label: common_maintainer(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'maintainer_ids',
				// The users who maintain some daemon.
				filterOptions: fieldValueOptions(ctx.maintainerValues()),
				getValue: (daemon) => ctx.maintainers().find((u) => u.id === daemon.user_id)?.email ?? null,
				getFilterValue: (daemon) => daemon.user_id ?? null,
				getGroupValue: (daemon) => daemon.user_id ?? null,
				filterLabel: (id) => ctx.maintainers().find((u) => u.id === id)?.email ?? id,
				display: {
					hiddenByDefault: true,
					getItems: (daemon) => {
						const user = ctx.maintainers().find((u) => u.id === daemon.user_id);
						if (!user) return [];
						return [
							{
								id: user.id,
								label: user.email,
								color: entities.getColorHelper('User').color,
								entityRef: entityRef('User', user.id, user)
							}
						];
					}
				}
			},
			// Sorted in version order on the server, so 0.17.10 follows 0.17.9.
			version: {
				filterable: true,
				serverFiltered: true,
				queryParam: 'versions',
				// The versions some daemon runs.
				filterOptions: fieldValueOptions(ctx.versionValues()),
				getGroupValue: (daemon) => daemon.version ?? null,
				getValue: (daemon) => daemon.version ?? '',
				display: { order: 4 }
			}
		},
		[
			{
				// One daemon per host, so the host neither groups nor filters; search finds it.
				key: 'host_id',
				label: common_host(),
				type: 'string',
				getValue: (daemon) => {
					const host = ctx.daemonHosts().find((h) => h.id === daemon.host_id);
					return host ? hostDisplayName(host) : common_unknownEntity({ entity: common_host() });
				},
				display: {
					hiddenByDefault: true,
					getItems: (daemon) => {
						const host = ctx.daemonHosts().find((h) => h.id === daemon.host_id);
						if (!host) return [];
						return [
							{
								id: host.id,
								label: hostDisplayName(host),
								color: entities.getColorHelper('Host').color,
								entityRef: entityRef('Host', host.id, host)
							}
						];
					}
				}
			},
			{
				// The address the server dials. A DaemonPoll daemon dials out instead, so
				// its stored url is unused and the cell stays empty.
				key: 'url',
				searchable: true,
				getValue: (daemon) => (daemon.mode === 'server_poll' ? daemon.url : null),
				display: { hiddenByDefault: true }
			},
			{
				key: 'interfaced_subnet_ids',
				type: 'array',
				// Daemons share subnets, so this filters; as an array it neither sorts nor groups.
				filterable: true,
				serverFiltered: true,
				filterOptions: ctx.subnets().map((subnet) => subnet.id),
				getValue: (daemon) => ctx.interfacedSubnets(daemon).map((s) => s.name),
				getFilterValue: (daemon) => ctx.interfacedSubnets(daemon).map((s) => s.id),
				filterLabel: (id) => ctx.subnets().find((s) => s.id === id)?.name ?? id,
				display: {
					order: 6,
					getItems: (daemon) =>
						ctx.interfacedSubnets(daemon).map((subnet) => ({
							id: subnet.id,
							label: subnet.name,
							color: entities.getColorHelper('Subnet').color,
							entityRef: entityRef('Subnet', subnet.id, subnet)
						}))
				}
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				filterable: true,
				getValue: (entity) => tagNames(entity.tags, ctx.tags())
			}
		]
	);
}
