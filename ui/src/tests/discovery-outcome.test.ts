import { describe, expect, it } from 'vitest';
import { runOutcomeTag } from '$lib/features/discovery/utils/outcome';
import { discoveryTerminalReasons } from '$lib/shared/stores/metadata';

describe('runOutcomeTag', () => {
	it('labels a recorded reason and explains it', () => {
		const tag = runOutcomeTag({ phase: 'Failed', reason: 'StalledNoUpdates' });
		expect(tag?.label).toBe(discoveryTerminalReasons.getName('StalledNoUpdates'));
		expect(tag?.title).toBe(discoveryTerminalReasons.getDescription('StalledNoUpdates'));
	});

	it('falls back to the phase for a run recorded before reasons existed', () => {
		const failed = runOutcomeTag({ phase: 'Failed', reason: null });
		expect(failed?.label).toBe(discoveryTerminalReasons.getName('DaemonReportedFailure'));
		// A legacy failure or cancellation may have been a stall; its cause is not known.
		expect(failed?.title).toBeUndefined();
		expect(runOutcomeTag({ phase: 'Cancelled' })?.title).toBeUndefined();
	});

	it('distinguishes a stall from a failure the daemon reported', () => {
		const stalled = runOutcomeTag({ phase: 'Failed', reason: 'StalledNoUpdates' });
		const failed = runOutcomeTag({ phase: 'Failed', reason: 'DaemonReportedFailure' });
		expect(stalled?.label).not.toBe(failed?.label);
	});

	it('shows a clean completion only where asked', () => {
		expect(runOutcomeTag({ phase: 'Complete', reason: 'Completed' })).toBeNull();
		expect(runOutcomeTag({ phase: 'Complete' })).toBeNull();
		expect(runOutcomeTag({ phase: 'Complete' }, { includeCompleted: true })).not.toBeNull();
	});

	it('names the stage of a run still in progress', () => {
		expect(runOutcomeTag({ phase: 'Scanning' })?.label).toBe('Scanning');
	});
});
