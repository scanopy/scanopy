import type { Credential, CredentialOrderField } from '../types/base';
import type { operations } from '$lib/api/schema';
import { entityRef, type CardFieldItem } from '$lib/shared/components/data/types';
import { defineRegistryFields } from '$lib/shared/entity-fields/table';
import type { EntityLookup } from '$lib/shared/entity-fields';
import type { Site } from '$lib/features/sites/types';
import type { Tag } from '$lib/features/tags/types/base';
import { credentialIntegrations, credentialTypes, entities } from '$lib/shared/stores/metadata';
import {
	getCredentialTypeId,
	getTargetTagProps,
	getUpstreamSupportTagProps
} from '$lib/features/credentials/types/base';
import { getStabilityTagProps } from '$lib/shared/utils/stability';
import type { Host } from '$lib/features/hosts/types/base';
import { hostDisplayName } from '$lib/features/hosts/host-display-name';
import osFamilies from '$lib/data/os-families.json';
import { credentials_betaTooltip, common_type, common_tags } from '$lib/paraglide/messages';
import { tagNames } from '$lib/features/tags/columns';

/** The integration a credential's type belongs to, as its `credentialIntegrations` id. */
function integrationId(item: Credential): string {
	return credentialTypes.getMetadata(getCredentialTypeId(item))?.integration ?? '';
}

/** Whether a credential's type is in beta. */
function isBeta(item: Credential): boolean {
	return credentialTypes.getMetadata(getCredentialTypeId(item))?.stability === 'Beta';
}

/** Whether a credential's type reads an API its vendor doesn't publish. */
function isUnofficialApi(item: Credential): boolean {
	return (
		credentialTypes.getMetadata(getCredentialTypeId(item))?.upstream_support === 'Undocumented'
	);
}

/** The credentials list endpoint's query params, which the filters are sent and linked as. */
type CredentialListParam = keyof NonNullable<operations['list_credentials']['parameters']['query']>;

/** What the credential columns read from the tab, each as a getter or function so values stay live. */
export interface CredentialColumnsContext {
	lookup: () => EntityLookup;
	tags: () => Tag[];
	sites: () => Site[];
	sitesForCredential: (credential: Credential) => Site[];
	hostsForCredential: (credential: Credential) => Host[];
	targetsFor: (credential: Credential) => string[];
	daemonHostOnly: (credential: Credential) => boolean;
	assignmentItems: (applicable: boolean, items: CardFieldItem[]) => CardFieldItem[];
	/** For an SSH login, which is assigned nowhere: how many credentials run over it. */
	loginUsage: (credential: Credential) => CardFieldItem[] | null;
}

/**
 * Columns from the entity field registry; this adds how each one searches, filters, sorts and
 * groups. Every CredentialOrderField value must be listed; the server orders, groups and filters
 * each, including the ones read from the type's metadata.
 */
export function credentialColumns(ctx: CredentialColumnsContext) {
	return defineRegistryFields<Credential, CredentialOrderField, CredentialListParam>(
		'Credential',
		ctx.lookup,
		{
			// Identity field: grouping by it would render a header per credential.
			name: {
				searchable: true,
				groupable: false,
				display: { primary: true, width: 220 }
			},
			created_at: { type: 'date', display: { hiddenByDefault: true } },
			updated_at: { type: 'date', display: { hiddenByDefault: true } },
			// Set only on credentials that read files or sockets on the daemon; empty otherwise.
			daemon_os: {
				filterable: true,
				serverFiltered: true,
				queryParam: 'daemon_os',
				// The field holds an OS family, so the options are every family.
				filterOptions: osFamilies.map((family) => family.id),
				filterLabel: (id) => osFamilies.find((family) => family.id === id)?.name ?? id,
				getFilterValue: (item: Credential) => item.daemon_os ?? null,
				getGroupValue: (item: Credential) => item.daemon_os ?? null,
				getValue: (item: Credential) =>
					item.daemon_os
						? (osFamilies.find((family) => family.id === item.daemon_os)?.name ?? item.daemon_os)
						: null,
				display: { hiddenByDefault: true }
			},
			// The integration a credential's type is a transport of (e.g. Docker for both the
			// socket and the proxy). The axis credentials are organized by.
			integration: {
				type: 'string',
				filterable: true,
				serverFiltered: true,
				queryParam: 'integrations',
				filterMode: 'include',
				filterOptions: credentialIntegrations.getItems().map((i) => i.id),
				filterLabel: (id) => credentialIntegrations.getName(id),
				getFilterValue: (item: Credential) => integrationId(item),
				// The server groups on the integration's id.
				getGroupValue: (item: Credential) => integrationId(item),
				getValue: (item: Credential) => credentialIntegrations.getName(integrationId(item)),
				display: {
					getItems: (item: Credential) => {
						const id = integrationId(item);
						return [
							{
								id,
								...credentialIntegrations.getTag(id),
								icon: credentialIntegrations.getIconComponent(id)
							}
						];
					}
				}
			},
			credential_type: {
				label: common_type(),
				type: 'string',
				filterable: true,
				serverFiltered: true,
				filterMode: 'include',
				queryParam: 'type',
				filterOptions: credentialTypes.getItems().map((t) => t.id),
				filterLabel: (id) => credentialTypes.getName(id),
				getFilterValue: (item: Credential) => getCredentialTypeId(item),
				// The server groups on the type's tag.
				getGroupValue: (item: Credential) => getCredentialTypeId(item),
				getValue: (item: Credential) => credentialTypes.getName(getCredentialTypeId(item)),
				display: {
					getItems: (item: Credential) => {
						const typeId = getCredentialTypeId(item);
						return [
							{
								id: typeId,
								...credentialTypes.getTag(typeId),
								icon: credentialTypes.getIconComponent(typeId)
							}
						];
					}
				}
			},
			// Beta and unofficial API are properties of the type, each with its own column so
			// either can be grouped and filtered on its own. A cell shows the tag the wizard and
			// the type dropdown use, and stays empty for a stable or vendor-supported type.
			beta: {
				type: 'boolean',
				filterable: true,
				serverFiltered: true,
				groupable: true,
				getGroupValue: (item: Credential) => String(isBeta(item)),
				getValue: (item: Credential) => isBeta(item),
				display: {
					getItems: (item: Credential) => {
						const typeId = getCredentialTypeId(item);
						const tag = getStabilityTagProps(
							credentialTypes.getMetadata(typeId)?.stability,
							credentials_betaTooltip()
						);
						return tag ? [{ id: `${typeId}-beta`, ...tag }] : [];
					}
				}
			},
			unofficial_api: {
				type: 'boolean',
				filterable: true,
				serverFiltered: true,
				groupable: true,
				getGroupValue: (item: Credential) => String(isUnofficialApi(item)),
				getValue: (item: Credential) => isUnofficialApi(item),
				display: {
					getItems: (item: Credential) => {
						const typeId = getCredentialTypeId(item);
						const tag = getUpstreamSupportTagProps(
							credentialTypes.getMetadata(typeId)?.upstream_support
						);
						return tag ? [{ id: `${typeId}-unofficial-api`, ...tag }] : [];
					}
				}
			}
		},
		[
			{ key: 'description', searchable: true },
			{
				key: 'assigned_sites',
				type: 'array',
				// Credentials share sites, so this filters. Hosts are near-unique per credential,
				// so search covers those.
				filterable: true,
				serverFiltered: true,
				queryParam: 'site_ids',
				filterOptions: ctx.sites().map((site) => site.id),
				getValue: (item: Credential) => ctx.sitesForCredential(item).map((n) => n.name),
				getFilterValue: (item: Credential) => ctx.sitesForCredential(item).map((n) => n.id),
				filterLabel: (id) => ctx.sites().find((n) => n.id === id)?.name ?? id,
				display: {
					getItems: (item: Credential) =>
						ctx.assignmentItems(
							ctx.targetsFor(item).includes('Site'),
							ctx.sitesForCredential(item).map((site) => ({
								id: site.id,
								label: site.name,
								color: entities.getColorHelper('Site').color,
								entityRef: entityRef('Site', site.id, site)
							}))
						)
				}
			},
			{
				key: 'assigned_hosts',
				type: 'array',
				searchable: true,
				getValue: (item: Credential) => ctx.hostsForCredential(item).map(hostDisplayName),
				display: {
					getItems: (item: Credential) =>
						ctx.loginUsage(item) ??
						ctx.assignmentItems(
							// DaemonHost-only credentials (Docker/Podman sockets) are
							// assigned to their daemon's host through the same junction.
							ctx.targetsFor(item).some((t) => t === 'Hosts' || t === 'DaemonHost'),
							ctx.hostsForCredential(item).map((host) => ({
								id: host.id,
								label: hostDisplayName(host),
								// A daemon-host credential reaches that host *through* its
								// daemon, so it reads as a daemon relationship rather than a
								// host one. Only credentials that target hosts generally get
								// the host colour.
								color: entities.getColorHelper(ctx.daemonHostOnly(item) ? 'Daemon' : 'Host').color,
								entityRef: entityRef('Host', host.id, host)
							}))
						)
				}
			},
			{
				key: 'target',
				type: 'array',
				filterable: true,
				serverFiltered: true,
				queryParam: 'targets',
				filterMode: 'include',
				// Every scope a credential type declares, read from the type metadata.
				filterOptions: [
					...new Set(
						credentialTypes
							.getItems()
							.flatMap((t) => credentialTypes.getMetadata(t.id).targets ?? [])
					)
				],
				getValue: (item: Credential) => {
					const typeId = getCredentialTypeId(item);
					const meta = credentialTypes.getMetadata(typeId);
					return meta?.targets ?? [];
				},
				display: {
					// Off by default: the target set is a property of the credential *type*, so it repeats
					// down the column for every credential of the same type and earns its width
					// only when someone is actually filtering by it. Still filterable. An array, so
					// it neither sorts nor groups.
					hiddenByDefault: true,
					// Coloured chips per target rather than undifferentiated grey.
					getItems: (item: Credential) => {
						const meta = credentialTypes.getMetadata(getCredentialTypeId(item));
						return (meta?.targets ?? []).map((target: string) => ({
							id: target,
							...getTargetTagProps(target)
						}));
					}
				}
			},
			{
				key: 'tags',
				label: common_tags(),
				type: 'array',
				searchable: true,
				filterable: true,
				getValue: (entity: Credential) => tagNames(entity.tags, ctx.tags())
			}
		]
	);
}
