import type { components } from '#lib/api/schema';
import {
	credentialIntegrations,
	credentialTypes,
	type CredentialTypeMetadata,
	type TypedTypeMetadata
} from '#lib/shared/stores/metadata.ts';
import { openModal, type ModalState } from '#lib/shared/stores/modal-registry.ts';
import { slugifySiteName } from '#lib/features/daemons/utils.ts';
import { credentials_createIntegrationCredential } from '#lib/paraglide/messages.js';

type CredentialIntegration = components['schemas']['CredentialIntegration'];
type TypeItem = Pick<TypedTypeMetadata<CredentialTypeMetadata>, 'id' | 'metadata'>;
type Target = NonNullable<CredentialTypeMetadata['targets']>[number];

/**
 * Open the credential create flow for an integration, assigned where it was found. The run report,
 * the home nudge and the digest email all land here, the email through the same URL params.
 */
export function openCredentialCreate(
	integration: CredentialIntegration,
	found: { hostIds: string[]; siteIds: string[] }
): void {
	window.location.hash = 'credentials';
	openModal('credential-editor', {
		prefill: prefillFor(credentialTypes.getItems() ?? [], integration, found)
	});
}

/**
 * Whether a new credential for this integration starts on the whole site rather than the hosts it
 * was found on: when one of its transports can apply to a site. A site-wide credential is tried on
 * every host and pinned where it works, which is what a mixed estate needs, since nothing says
 * which community or version a device accepts until one answers. The other integrations'
 * credentials can only be pinned to hosts. Mirrors `CredentialIntegration::applies_to_sites`.
 */
export function prefillsSites(types: TypeItem[], integration: string): boolean {
	return typesForIntegration(types, integration, 'Site').length > 0;
}

/** The URL params that open the create flow for an integration, assigned where it was found. */
export function prefillFor(
	types: TypeItem[],
	integration: CredentialIntegration,
	{ hostIds, siteIds }: { hostIds: string[]; siteIds: string[] }
): NonNullable<ModalState['prefill']> {
	const join = (ids: string[]) => (ids.length ? ids.join(',') : undefined);
	return prefillsSites(types, integration)
		? { integration, sites: join(siteIds) }
		: { integration, hosts: join(hostIds) };
}

/**
 * The credential types the create flow offers for a pre-filled integration: its transports, and
 * only those that can apply to `target` when the flow comes pre-assigned there (a Docker socket
 * reaches only the daemon's own host). Fixture order, so the first is the one pre-selected.
 */
export function typesForIntegration(
	types: TypeItem[],
	integration: string,
	target: Target | null
): string[] {
	return types
		.filter((t) => t.metadata?.integration === integration)
		.filter((t) => !target || (t.metadata?.targets ?? []).includes(target))
		.map((t) => t.id);
}

/**
 * The name a pre-filled credential starts with, snake_cased: the host's own name when it is for
 * one host, the integration's when it is for several (one host's name would mislabel the rest), for
 * a site, or for a host with no name of its own.
 */
export function prefillCredentialName(
	hosts: { display_name?: string | null }[],
	integrationName: string
): string {
	const hostName = hosts.length === 1 ? hosts[0].display_name?.trim() : undefined;
	return slugifySiteName(hostName || integrationName, '_');
}

/** The button that opens the create flow: "Create Proxmox VE Credential". */
export function createIntegrationCredentialLabel(integration: CredentialIntegration): string {
	return credentials_createIntegrationCredential({
		integration: credentialIntegrations.getName(integration)
	});
}
