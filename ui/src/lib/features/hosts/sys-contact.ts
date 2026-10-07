import { validate } from 'email-validator';

/**
 * The email address inside an SNMP sysContact, or `null` when it holds none.
 *
 * sysContact is free text: a bare address, a name with one ("Network Ops <ops@example.com>"),
 * a `mailto:` URI, or no address at all ("John Doe, ext. 4410"). The address is the first token
 * that validates as one, so a contact can link to it while still showing as written.
 */
export function sysContactEmail(contact: string | null | undefined): string | null {
	if (!contact) return null;
	for (const token of contact.split(/[\s<>()[\],;"']+/)) {
		const candidate = token.replace(/^mailto:/i, '');
		if (candidate && validate(candidate)) return candidate;
	}
	return null;
}
