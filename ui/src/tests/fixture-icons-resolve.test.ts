import { describe, expect, it } from 'vitest';
import * as LucideIcons from 'lucide-svelte';
import { createIconComponent } from '$lib/shared/utils/styling';

// Icon names come from the backend's `lucide-icons` crate, and the UI renders them through
// lucide-svelte. When the two packages are on different versions, a renamed icon falls back to
// the `?` icon with no error. Network Identities rendered `?` this way: `fingerprint-pattern`
// came from lucide-icons 0.562 and did not exist in lucide-svelte 0.526.
const fixtures = import.meta.glob<unknown>('$lib/data/*.json', { eager: true, import: 'default' });

function collectIcons(value: unknown, path: string, out: Array<[string, string]>) {
	if (Array.isArray(value)) {
		value.forEach((v, i) => collectIcons(v, `${path}[${i}]`, out));
	} else if (value && typeof value === 'object') {
		for (const [key, v] of Object.entries(value)) {
			if (key === 'icon' && typeof v === 'string') out.push([`${path}.${key}`, v]);
			else collectIcons(v, `${path}.${key}`, out);
		}
	}
}

describe('fixture icons', () => {
	it('every icon the backend emits resolves to a lucide-svelte component', () => {
		const icons: Array<[string, string]> = [];
		for (const [file, data] of Object.entries(fixtures)) {
			collectIcons(data, file.split('/').pop()!, icons);
		}
		expect(icons.length).toBeGreaterThan(0);

		const unresolved = icons.filter(
			([, name]) =>
				createIconComponent(name) === LucideIcons.HelpCircle &&
				!['circle-question-mark', 'help-circle', 'circle-help'].includes(name)
		);
		expect(unresolved).toEqual([]);
	});
});
