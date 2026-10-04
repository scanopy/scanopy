import { describe, it, expect } from 'vitest';
import { sysContactEmail } from '$lib/features/hosts/sys-contact';

describe('sysContactEmail', () => {
	it('finds the address in the forms sysContact takes', () => {
		expect(sysContactEmail('ops@example.com')).toBe('ops@example.com');
		expect(sysContactEmail('Network Ops <ops@example.com>')).toBe('ops@example.com');
		expect(sysContactEmail('mailto:ops@example.com')).toBe('ops@example.com');
		expect(sysContactEmail('Jane (jane@example.com), ext. 12')).toBe('jane@example.com');
	});

	it('returns null when the contact holds no address', () => {
		expect(sysContactEmail('John Doe, ext. 4410')).toBe(null);
		expect(sysContactEmail('admin@localhost')).toBe(null);
		expect(sysContactEmail('')).toBe(null);
		expect(sysContactEmail(null)).toBe(null);
	});
});
