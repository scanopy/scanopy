import type { components } from '#lib/api/schema';
import type { TagProps } from '#lib/shared/components/data/types.ts';
import type { Color } from '#lib/shared/utils/styling.ts';
import { common_beta } from '#lib/paraglide/messages.js';

/** Release maturity of a credential type or a feature. Derived from the backend enum. */
export type Stability = components['schemas']['Stability'];

/**
 * Tag marking something still in beta, or `null` when it is stable. `title` says what beta means
 * where the tag sits; `href` makes the tag a link, as a beta feature's links to the beta program.
 */
export function getStabilityTagProps(
	stability: Stability | undefined,
	title: string,
	href?: string
): TagProps | null {
	if (stability !== 'Beta') return null;
	return {
		label: common_beta(),
		// Purple: amber is the paid crown's and the Host target's, so Beta stands apart from both.
		color: 'Purple' as Color,
		title,
		href
	};
}
