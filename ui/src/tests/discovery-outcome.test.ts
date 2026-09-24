import { describe, expect, it } from 'vitest';
import { runOutcomeReason, runOutcomeTag } from '$lib/features/discovery/utils/outcome';
import { discoveryPhases, discoveryTerminalReasons } from '$lib/shared/stores/metadata';

describe('runOutcomeTag', () => {
	it('labels the outcome, not the reason behind it', () => {
		const stalled = runOutcomeTag({ phase: 'Failed', reason: 'StalledNoUpdates' });
		const restarted = runOutcomeTag({ phase: 'Failed', reason: 'DaemonRestarted' });

		expect(stalled?.label).toBe(discoveryPhases.getName('Failed'));
		expect(restarted?.label).toBe(stalled?.label);
		expect(stalled?.color).toBe(restarted?.color);
	});

	it('carries the reason and what to check as the hover', () => {
		const tag = runOutcomeTag({ phase: 'Failed', reason: 'StalledNoUpdates' });

		expect(tag?.title).toContain(discoveryTerminalReasons.getName('StalledNoUpdates'));
		expect(tag?.title).toContain(discoveryTerminalReasons.getDescription('StalledNoUpdates'));
	});

	it('leaves a run recorded before reasons existed with its status alone', () => {
		const failed = runOutcomeTag({ phase: 'Failed', reason: null });

		expect(failed?.label).toBe(discoveryPhases.getName('Failed'));
		expect(failed?.title).toBeUndefined();
		expect(runOutcomeTag({ phase: 'Cancelled' })?.title).toBeUndefined();
	});

	it('shows a clean completion only where asked', () => {
		expect(runOutcomeTag({ phase: 'Complete', reason: 'Completed' })).toBeNull();
		expect(runOutcomeTag({ phase: 'Complete' })).toBeNull();
		expect(runOutcomeTag({ phase: 'Complete' }, { includeCompleted: true })?.label).toBe(
			discoveryPhases.getName('Complete')
		);
	});

	it('names the stage of a run still in progress', () => {
		expect(runOutcomeTag({ phase: 'Scanning' })?.label).toBe('Scanning');
	});
});

describe('runOutcomeReason', () => {
	it('distinguishes a stall from a failure the daemon reported', () => {
		const stalled = runOutcomeReason({ phase: 'Failed', reason: 'StalledNoUpdates' });
		const reported = runOutcomeReason({ phase: 'Failed', reason: 'DaemonReportedFailure' });

		expect(stalled?.name).not.toBe(reported?.name);
		expect(stalled?.description).toBeTruthy();
	});

	it('has nothing to explain for a run recorded before reasons existed', () => {
		expect(runOutcomeReason({ phase: 'Failed', reason: null })).toBeNull();
	});
});
