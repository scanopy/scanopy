import { discoveryTerminalReasons } from '$lib/shared/stores/metadata';
import { toColor, type Color } from '$lib/shared/utils/styling';
import type { DiscoveryTerminalReason, DiscoveryUpdatePayload } from '../types/api';

export interface OutcomeTag {
	label: string;
	color: Color;
	/** What the reason means and what to check. Absent for rows recorded before reasons existed. */
	title?: string;
}

/**
 * The reason a row recorded before reasons existed stands in for, for its label and colour only.
 * Its description would be a guess: a stall used to be recorded as a cancellation.
 */
const LEGACY_REASON: Partial<Record<DiscoveryUpdatePayload['phase'], DiscoveryTerminalReason>> = {
	Complete: 'Completed',
	Cancelled: 'UserCancelled',
	Failed: 'DaemonReportedFailure'
};

/**
 * How a run ended, as a tag, or `null` when there is nothing to show. A clean completion shows
 * only where `includeCompleted` asks for it; elsewhere the point is that the other outcomes stand
 * out.
 */
export function runOutcomeTag(
	results: Pick<DiscoveryUpdatePayload, 'phase' | 'reason'> | null | undefined,
	{ includeCompleted = false }: { includeCompleted?: boolean } = {}
): OutcomeTag | null {
	const phase = results?.phase;
	if (!phase) return null;

	const recorded = results.reason ?? null;
	const reason = recorded ?? LEGACY_REASON[phase];
	// Still running, so worth showing: the phase names its stage.
	if (!reason) return { label: phase, color: toColor('blue') };
	if (reason === 'Completed' && !includeCompleted) return null;

	return {
		label: discoveryTerminalReasons.getName(reason),
		color: discoveryTerminalReasons.getColorString(reason),
		title: recorded ? discoveryTerminalReasons.getDescription(recorded) : undefined
	};
}
