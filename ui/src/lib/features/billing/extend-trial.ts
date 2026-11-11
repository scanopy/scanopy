import { queryClient, queryKeys } from '#lib/api/query-client.ts';
import { useExtendTrialMutation } from '#lib/features/billing/queries.ts';
import { waitForOrgUpdate } from '#lib/shared/billing/wait-for-org-update.ts';
import { pushSuccess, pushWarning } from '#lib/shared/stores/feedback.ts';
import {
	billing_requestAccepted,
	billing_trialExtended,
	settings_billing_extendTrial_confirmBody
} from '#lib/paraglide/messages.js';

/**
 * The extend action shared by the Billing tab and the trial banner: confirm,
 * call the endpoint, then wait for the webhook to mark the extension used.
 * Call during component setup.
 */
export function useExtendTrial() {
	const mutation = useExtendTrialMutation();

	async function extend() {
		if (!confirm(settings_billing_extendTrial_confirmBody())) return;
		try {
			await mutation.mutateAsync();
			const flipped = await waitForOrgUpdate((o) => o.trial_extended_used === true);
			if (flipped) {
				pushSuccess(billing_trialExtended());
			} else {
				pushWarning(billing_requestAccepted());
			}
			await queryClient.invalidateQueries({ queryKey: queryKeys.organizations.current() });
		} catch {
			// Mutation onError handles toast.
		}
	}

	return {
		extend,
		get isPending() {
			return mutation.isPending;
		}
	};
}
