import { entityRef, type CardFieldItem } from '$lib/shared/components/data/types';
import { entities } from '$lib/shared/stores/metadata';
import type { Credential } from './types/base';

/**
 * Credentials as navigable chips.
 *
 * The colour is the Credential entity's, not the credential type's. Colouring
 * by type made the same credential render differently depending on the list it
 * appeared in — a site's SSH credential and a host's SNMP credential looked
 * like unrelated types of thing, when what the chip says is "this is a
 * credential, here it is". Type is a column on the credentials tab, which is
 * where the type colours belong.
 *
 * Takes ids or the credentials themselves, since callers hold either.
 */
export function credentialItems(
	credentials: (Credential | undefined)[],
	all?: never
): CardFieldItem[];
export function credentialItems(
	credentialIds: string[] | null | undefined,
	all: Credential[]
): CardFieldItem[];
export function credentialItems(
	input: (Credential | undefined)[] | string[] | null | undefined,
	all?: Credential[]
): CardFieldItem[] {
	if (!input) return [];

	const resolved = (input as (Credential | string | undefined)[]).map((entry) =>
		typeof entry === 'string' ? all?.find((c) => c.id === entry) : entry
	);

	return resolved
		.filter((credential): credential is Credential => Boolean(credential))
		.map((credential) => ({
			id: credential.id,
			label: credential.name,
			color: entities.getColorHelper('Credential').color,
			entityRef: entityRef('Credential', credential.id, credential)
		}));
}
