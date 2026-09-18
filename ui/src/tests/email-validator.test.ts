import { describe, it, expect } from 'vitest';
import { email } from '$lib/shared/components/forms/validators';

/**
 * Surrounding whitespace on a pasted address.
 *
 * The backend parses emails with the `email_address` crate, which stores the string verbatim and
 * rejects a leading or trailing space outright: `" a@b.com"` fails the local-part character check
 * and `"a@b.com "` fails the domain label's trailing-alphanumeric check. So whitespace never
 * reaches the database either way, and the only question is whether the user gets a usable signup
 * or a 400.
 *
 * The auth forms answer that by trimming at submit. This validator runs earlier, on blur, against
 * the raw field value, so before it trimmed it would mark a pasted address invalid while the very
 * next step accepted it happily. Trimming here keeps the two halves agreeing.
 */
describe('email validator', () => {
	it('accepts an address pasted with surrounding whitespace', () => {
		expect(email(' user@example.com ')).toBeUndefined();
		expect(email('\tuser@example.com\n')).toBeUndefined();
	});

	it('still rejects an address that is malformed once trimmed', () => {
		// Trimming must not become a way to smuggle a bad address past the check.
		expect(email(' user@ ')).toBeDefined();
		expect(email(' not-an-email ')).toBeDefined();
	});

	it('leaves interior whitespace invalid', () => {
		// Only the edges are incidental to a paste; a space in the middle is a different address.
		expect(email('user name@example.com')).toBeDefined();
	});
});
