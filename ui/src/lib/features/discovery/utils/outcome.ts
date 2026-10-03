import { discoveryPhases, discoveryTerminalReasons } from '$lib/shared/stores/metadata';
import { toColor, type Color } from '$lib/shared/utils/styling';
import type { DiscoveryUpdatePayload } from '../types/api';

export interface OutcomeTag {
	label: string;
	color: Color;
	/** The reason, as a hover. Absent for a row recorded before reasons existed. */
	title?: string;
}

/** The reason a run ended, for the surfaces that explain rather than label. */
export interface OutcomeReason {
	name: string;
	description: string;
}

/** The phases a run ends in, as the backend's `DiscoveryPhase::is_terminal` defines them. Fixture
 *  ids are `DiscoveryPhase` names, emitted from that enum, so each is a valid phase by construction. */
export const TERMINAL_PHASES = discoveryPhases
	.getItems()
	.filter((phase) => discoveryPhases.getMetadata(phase.id).is_terminal)
	.map((phase) => phase.id as DiscoveryUpdatePayload['phase']);

/**
 * How a run ended: one of three statuses, with the reason behind it as the hover.
 *
 * A run has one outcome and many ways of arriving at it, and the two are not interchangeable.
 * "Daemon restarted" is why a scan failed, not a fourth thing a scan can be, so it belongs in the
 * explanation rather than in the label a list is scanned by.
 *
 * `null` when there is nothing to show: a clean completion, unless `includeCompleted` asks for it,
 * since elsewhere the point is that the other outcomes stand out.
 */
export function runOutcomeTag(
	results: Pick<DiscoveryUpdatePayload, 'phase' | 'reason'> | null | undefined,
	{ includeCompleted = false }: { includeCompleted?: boolean } = {}
): OutcomeTag | null {
	const phase = results?.phase;
	if (!phase) return null;
	// Still running, so worth showing: the phase names its stage.
	if (!TERMINAL_PHASES.includes(phase)) return { label: phase, color: toColor('blue') };
	if (phase === 'Complete' && !includeCompleted) return null;

	const reason = runOutcomeReason(results);
	return {
		label: discoveryPhases.getName(phase),
		color: discoveryPhases.getColorString(phase),
		title: reason ? `${reason.name}: ${reason.description}` : undefined
	};
}

/**
 * Why a run ended the way it did, or `null` for a run still going or one recorded before reasons
 * existed. A clean completion has a reason like any other, for surfaces that show one.
 */
export function runOutcomeReason(
	results: Pick<DiscoveryUpdatePayload, 'phase' | 'reason'> | null | undefined
): OutcomeReason | null {
	const reason = results?.reason;
	if (!reason) return null;
	return {
		name: discoveryTerminalReasons.getName(reason),
		description: discoveryTerminalReasons.getDescription(reason)
	};
}
