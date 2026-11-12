import PaidCrown from '#lib/shared/components/PaidCrown.svelte';
import type { TagProps } from '#lib/shared/components/data/types.ts';
import { common_availableOnPaidPlans } from '#lib/paraglide/messages.js';

/**
 * The badge on something the plan lacks (an export format, a credential type, snapshots): the
 * crown alone. Its tooltip says the feature is paid without naming a plan, since the lowest one
 * differs between cloud and self-hosted.
 */
export function upgradeTagProps(title: string = common_availableOnPaidPlans()): TagProps {
	return { icon: PaidCrown, upgrade: true, title };
}
