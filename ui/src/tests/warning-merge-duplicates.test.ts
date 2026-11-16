import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('#lib/shared/stores/modal-registry.ts', () => ({
	openModal: vi.fn(),
	closeModal: vi.fn(),
	navigateToEntity: vi.fn()
}));

import { closeModal, openModal } from '#lib/shared/stores/modal-registry.ts';
import { actionFor, duplicateGroupsOf } from '#lib/features/discovery/utils/warningActions.ts';
import { hostIdsOf, type WarningEntry } from '#lib/features/discovery/utils/warnings.ts';

const run = { daemonId: 'd', siteId: 's', scanId: null, credentialOf: () => undefined };

/** An ambiguity row whose neighbours each name `candidates`, seen from `local`. */
function ambiguityRow(...neighbours: { local: string; candidates: string[] }[]): WarningEntry {
	return {
		code: 'LldpNeighbourAmbiguous',
		warnings: neighbours.map(({ local, candidates }) => ({
			code: 'LldpNeighbourAmbiguous',
			host_id: local,
			if_descr: 'Gi0/1',
			identifier: '00:04:38:02:e0:00',
			sys_name: null,
			address: null,
			candidate_host_ids: candidates
		}))
	} as unknown as WarningEntry;
}

describe('Merge duplicate hosts', () => {
	beforeEach(() => {
		vi.mocked(openModal).mockClear();
		vi.mocked(closeModal).mockClear();
		vi.stubGlobal('window', { location: { hash: '' } });
	});

	it('opens the merge with the newer record merging into the oldest', () => {
		// Two switches name the same pair: one group.
		const row = ambiguityRow(
			{ local: 'sw-a', candidates: ['oldest', 'newer'] },
			{ local: 'sw-b', candidates: ['newer', 'oldest'] }
		);
		expect(duplicateGroupsOf(row)).toEqual([['oldest', 'newer']]);

		actionFor(row, run)?.run();
		expect(window.location.hash).toBe('hosts');
		expect(openModal).toHaveBeenCalledWith('host-merge', {
			id: 'newer',
			subEntityId: 'oldest'
		});
	});

	it('goes to the host list when the row covers several duplicate groups', () => {
		const row = ambiguityRow(
			{ local: 'sw-a', candidates: ['a1', 'a2'] },
			{ local: 'sw-b', candidates: ['b1', 'b2'] }
		);

		actionFor(row, run)?.run();
		expect(openModal).not.toHaveBeenCalled();
		expect(closeModal).toHaveBeenCalled();
	});

	it('offers nothing for a warning stored before candidates were carried', () => {
		expect(actionFor(ambiguityRow({ local: 'sw-a', candidates: [] }), run)).toBeNull();
	});

	it('gives the row a chip for each candidate host, once each', () => {
		const row = ambiguityRow(
			{ local: 'sw-a', candidates: ['oldest', 'newer'] },
			{ local: 'sw-b', candidates: ['oldest', 'newer'] }
		);
		expect(hostIdsOf(row)).toEqual(['sw-a', 'oldest', 'newer', 'sw-b']);
	});
});
