/**
 * Whether a subnet's range is one Scanopy assumed and nobody has confirmed.
 *
 * A subnet is unconfirmed when its range was assumed (`source` is `Inferred`), and the verdict is
 * the shared one in `confirmation.ts`, the same a host carries. Named for the range because that
 * is what its callers ask: whether a range can nest, be merged into, or offer "Resolve range".
 */

import { isUnconfirmed, type ConfirmationSubject } from '#lib/shared/utils/confirmation.ts';

/** Whether this subnet's range is a guess awaiting confirmation. */
export function isProvisionalCidr(subnet: ConfirmationSubject): boolean {
	return isUnconfirmed(subnet);
}
