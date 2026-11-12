import { describe, it, expect } from 'vitest';
import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
import { hosts_unnamedHost } from '#lib/paraglide/messages.js';

/**
 * The frontend half of the title contract.
 *
 * The ladder itself is `Host::display_name` and is tested in Rust — asserting its rungs again here
 * would be a second copy of the thing this whole change exists to remove. What is genuinely
 * frontend behaviour is the last step: `display_name` is `Option<String>` on the wire, and every
 * surface has to render *one* deliberate thing when it is absent rather than each inventing its
 * own `?? ''`.
 */
describe('hostDisplayName', () => {
	it('renders the title the server resolved', () => {
		expect(hostDisplayName({ display_name: 'core-sw-01' })).toBe('core-sw-01');
	});

	it('falls back for a host with nothing on any rung', () => {
		// `skip_serializing_if = "Option::is_none"` means the key is *absent*, not null — both
		// shapes reach the frontend depending on how the object was built, and neither may render
		// as an empty title.
		expect(hostDisplayName({})).toBe(hosts_unnamedHost());
		expect(hostDisplayName({ display_name: null })).toBe(hosts_unnamedHost());
	});

	it('treats a blank title as no title', () => {
		// A row or a node titled with whitespace looks like a rendering bug, and is one. The
		// backend returns `None` rather than `Some("")` precisely so this state is expressible;
		// the guard is here too because the fallback must not depend on that staying true.
		expect(hostDisplayName({ display_name: '' })).toBe(hosts_unnamedHost());
		expect(hostDisplayName({ display_name: '   ' })).toBe(hosts_unnamedHost());
	});

	it('never returns an empty string', () => {
		// The property the surfaces actually rely on: a caller can drop the result straight into a
		// cell, a node header or a confirm dialog without a fallback of its own.
		for (const display_name of [undefined, null, '', '  ', 'web-01']) {
			expect(hostDisplayName({ display_name }).length).toBeGreaterThan(0);
		}
	});
});
