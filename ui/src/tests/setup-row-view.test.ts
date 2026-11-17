import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('#lib/shared/stores/modal-registry.ts', () => ({
	openModal: vi.fn(),
	closeModal: vi.fn(),
	navigateToEntity: vi.fn()
}));
vi.mock('#lib/shared/stores/url-state.ts', () => ({ openTab: vi.fn() }));

import { openModal } from '#lib/shared/stores/modal-registry.ts';
import { openTab } from '#lib/shared/stores/url-state.ts';
import { setupRowView, type SetupRowContext } from '#lib/features/home/setup-row-view.ts';
import type { SetupItem } from '#lib/features/home/setup-list.ts';

const DAEMON = '00000000-0000-4000-8000-000000000001';
const SITE = '00000000-0000-4000-8000-000000000002';

const ctx: SetupRowContext = {
	setup: [],
	lookups: {
		daemon: () => undefined,
		host: () => undefined,
		site: () => undefined,
		credential: () => undefined,
		scan: () => undefined
	},
	onAnswer: vi.fn(),
	onTroubleshoot: vi.fn(),
	onRetryScans: vi.fn()
};

function click(item: SetupItem) {
	setupRowView(item, ctx).fix.run();
}

/** The tab and list params the last `openTab` call opened. */
function openedTab() {
	const [tab, params] = vi.mocked(openTab).mock.lastCall ?? [];
	return { tab, params: Object.fromEntries(params ?? []) };
}

describe('Setup row destinations', () => {
	beforeEach(() => {
		vi.mocked(openModal).mockClear();
		vi.mocked(openTab).mockClear();
		vi.stubGlobal('window', { location: { hash: '' } });
	});

	it('sends the Tags step to the hosts list, where a tag gets assigned', () => {
		click({ type: 'Tags' });
		expect(window.location.hash).toBe('hosts');
		expect(openModal).not.toHaveBeenCalled();
	});

	it('sends a daemon on standby to its own scans, whose Run wakes it', () => {
		click({
			type: 'DaemonHealth',
			daemon_id: DAEMON,
			name: 'edge-01',
			site_id: SITE,
			health: 'standby'
		});
		expect(openedTab()).toEqual({ tab: 'discovery-scans', params: { daemon_ids: DAEMON } });
	});

	it("sends the first scan step to the first daemon's scans", () => {
		click({ type: 'RunFirstScan', daemon_id: DAEMON });
		expect(openedTab()).toEqual({ tab: 'discovery-scans', params: { daemon_ids: DAEMON } });
	});

	it('sends the first scan step to every scan when no daemon has connected', () => {
		click({ type: 'RunFirstScan', daemon_id: null });
		expect(openTab).not.toHaveBeenCalled();
		expect(window.location.hash).toBe('discovery-scans');
	});
});

describe('Failed scan rows', () => {
	const SCAN = '00000000-0000-4000-8000-000000000003';
	const RUN = '00000000-0000-4000-8000-000000000004';
	const failed = (
		reason: 'WatchdogTimeout' | 'SessionSilent' | 'DaemonReportedFailure',
		nextRunFull: boolean
	): SetupItem => ({
		type: 'FailedScans',
		daemon_id: DAEMON,
		daemon_name: 'edge-01',
		site_id: SITE,
		reason,
		scans: [{ scan_id: SCAN, name: 'Office LAN', run_id: RUN, next_run_full: nextRunFull }]
	});

	beforeEach(() => {
		vi.mocked(openModal).mockClear();
		vi.stubGlobal('window', { location: { hash: '' } });
	});

	it("sends a timed-out scan to its editor's Performance tab, with the layout guide", () => {
		const view = setupRowView(failed('WatchdogTimeout', false), ctx);
		view.fix.run();
		expect(openModal).toHaveBeenCalledWith('discovery-editor', { id: SCAN, tab: 'performance' });
		expect(view.docsUrl).toContain('#choose-a-layout');
	});

	it('links a silent scan to the connection troubleshooting guide', () => {
		const view = setupRowView(failed('SessionSilent', false), ctx);
		view.fix.run();
		expect(ctx.onTroubleshoot).toHaveBeenCalled();
		expect(view.docsUrl).toContain('#daemon-not-connecting-to-server');
	});

	it('says a full scan runs full again only when one in the row was full', () => {
		const light = setupRowView(failed('DaemonReportedFailure', false), ctx);
		const full = setupRowView(failed('DaemonReportedFailure', true), ctx);
		expect(full.sentence.startsWith(light.sentence)).toBe(true);
		expect(full.sentence.length).toBeGreaterThan(light.sentence.length);
		expect(light.docsUrl).toBeUndefined();
	});
});
