import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('#lib/api/client.ts', () => ({
	isRateLimited: () => false,
	getRateLimitDelay: () => 0
}));

import { BaseSSEManager, HIDDEN_GRACE_MS, type SSEConfig } from '#lib/shared/utils/sse.ts';

class FakeEventSource {
	static CONNECTING = 0;
	static OPEN = 1;
	static CLOSED = 2;
	static instances: FakeEventSource[] = [];

	readyState = FakeEventSource.CONNECTING;
	onopen: (() => void) | null = null;
	onmessage: ((event: { data: string }) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;

	constructor(public url: string) {
		FakeEventSource.instances.push(this);
	}

	open() {
		this.readyState = FakeEventSource.OPEN;
		this.onopen?.();
	}

	fail() {
		this.onerror?.(new Event('error'));
	}

	close() {
		this.readyState = FakeEventSource.CLOSED;
	}

	static live() {
		return FakeEventSource.instances.filter((es) => es.readyState !== FakeEventSource.CLOSED);
	}
}

class FakeDocument extends EventTarget {
	visibilityState: 'visible' | 'hidden' = 'visible';

	setVisibility(state: 'visible' | 'hidden') {
		this.visibilityState = state;
		this.dispatchEvent(new Event('visibilitychange'));
	}
}

class TestManager extends BaseSSEManager<unknown> {
	reconnected = 0;

	protected createConfig(): SSEConfig<unknown> {
		return { url: '/stream', onMessage: () => {} };
	}

	protected override onReconnected() {
		this.reconnected++;
	}
}

let doc: FakeDocument;
let manager: TestManager;

beforeEach(() => {
	vi.useFakeTimers();
	FakeEventSource.instances = [];
	doc = new FakeDocument();
	vi.stubGlobal('EventSource', FakeEventSource);
	vi.stubGlobal('document', doc);
	vi.spyOn(console, 'error').mockImplementation(() => {});
	vi.spyOn(console, 'log').mockImplementation(() => {});
	manager = new TestManager();
});

afterEach(() => {
	manager.disconnect();
	vi.useRealTimers();
	vi.unstubAllGlobals();
	vi.restoreAllMocks();
});

describe('BaseSSEManager', () => {
	it('opens one EventSource however often connect() is called', () => {
		manager.connect();
		manager.connect();
		FakeEventSource.instances[0].open();
		manager.connect();

		expect(FakeEventSource.instances).toHaveLength(1);
	});

	it('opens no extra EventSource while a reconnect is pending', () => {
		manager.connect();
		FakeEventSource.instances[0].fail();
		manager.connect();
		vi.advanceTimersByTime(60_000);

		expect(FakeEventSource.instances).toHaveLength(2);
		expect(FakeEventSource.live()).toHaveLength(1);
	});

	it('cancels a pending reconnect on disconnect()', () => {
		manager.connect();
		FakeEventSource.instances[0].fail();
		manager.disconnect();
		vi.advanceTimersByTime(60_000);

		expect(FakeEventSource.instances).toHaveLength(1);
		expect(FakeEventSource.live()).toHaveLength(0);
	});

	it('closes the stream in a tab hidden past the grace period and reopens it on return', () => {
		manager.connect();
		FakeEventSource.instances[0].open();

		doc.setVisibility('hidden');
		vi.advanceTimersByTime(HIDDEN_GRACE_MS);
		expect(FakeEventSource.live()).toHaveLength(0);

		doc.setVisibility('visible');
		expect(FakeEventSource.live()).toHaveLength(1);
		FakeEventSource.instances[1].open();
		expect(manager.reconnected).toBe(1);
	});

	it('keeps the stream through a tab switch shorter than the grace period', () => {
		manager.connect();
		FakeEventSource.instances[0].open();

		doc.setVisibility('hidden');
		vi.advanceTimersByTime(HIDDEN_GRACE_MS - 1);
		doc.setVisibility('visible');
		vi.advanceTimersByTime(HIDDEN_GRACE_MS);

		expect(FakeEventSource.instances).toHaveLength(1);
		expect(FakeEventSource.live()).toHaveLength(1);
		expect(manager.reconnected).toBe(0);
	});

	it('stays closed in a hidden tab after disconnect()', () => {
		manager.connect();
		manager.disconnect();
		doc.setVisibility('hidden');
		doc.setVisibility('visible');

		expect(FakeEventSource.live()).toHaveLength(0);
	});
});
