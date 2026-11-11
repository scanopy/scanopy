import { isRateLimited, getRateLimitDelay } from '#lib/api/client.ts';

export interface SSEConfig<T> {
	url: string;
	onMessage: (data: T) => void;
	onError?: (error: Event) => void;
	onOpen?: () => void;
}

const INITIAL_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 30_000;

/**
 * How long a hidden tab keeps its stream. Over HTTP/1.1 every open stream holds one of the
 * browser's six connections to the host, shared by all tabs, so a hidden tab gives its up;
 * the grace period keeps a quick tab switch from churning the connection.
 */
export const HIDDEN_GRACE_MS = 10_000;

/**
 * One EventSource with its own backoff reconnect. On an error the EventSource is closed, so its
 * built-in reconnect never runs alongside this one.
 */
export class SSEClient<T> {
	private eventSource: EventSource | null = null;
	private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
	private reconnectAttempts = 0;

	constructor(private config: SSEConfig<T>) {}

	/** Opens the stream. A no-op while it is connecting, open, or waiting to reconnect. */
	connect(): void {
		if (this.eventSource || this.reconnectTimer) return;

		const eventSource = new EventSource(this.config.url);
		this.eventSource = eventSource;

		eventSource.onopen = () => {
			this.reconnectAttempts = 0;
			this.config.onOpen?.();
		};

		eventSource.onmessage = async (event) => {
			try {
				const data = JSON.parse(event.data) as T;
				await this.config.onMessage(data);
			} catch (error) {
				console.error('Failed to parse SSE message:', error);
			}
		};

		eventSource.onerror = (error) => {
			console.error('SSE error:', error);
			eventSource.close();
			this.eventSource = null;
			this.config.onError?.(error);
			this.scheduleReconnect();
		};
	}

	disconnect(): void {
		if (this.reconnectTimer) {
			clearTimeout(this.reconnectTimer);
			this.reconnectTimer = null;
		}
		this.reconnectAttempts = 0;
		if (this.eventSource) {
			this.eventSource.close();
			this.eventSource = null;
		}
	}

	isConnected(): boolean {
		return this.eventSource?.readyState === EventSource.OPEN;
	}

	private scheduleReconnect() {
		let delay = Math.min(
			INITIAL_RECONNECT_DELAY_MS * 2 ** this.reconnectAttempts,
			MAX_RECONNECT_DELAY_MS
		);
		this.reconnectAttempts++;
		// If globally rate-limited, wait for the rate limit window to pass
		if (isRateLimited()) {
			delay += getRateLimitDelay();
		}
		console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

		this.reconnectTimer = setTimeout(() => {
			this.reconnectTimer = null;
			this.connect();
		}, delay);
	}
}

/**
 * Base SSE manager class that handles connection lifecycle: one stream while the tab is visible,
 * none once it has been hidden for `HIDDEN_GRACE_MS`.
 * Extend this for specific SSE use cases
 */
export abstract class BaseSSEManager<T> {
	private client: SSEClient<T> | null = null;
	private hiddenTimer: ReturnType<typeof setTimeout> | null = null;
	private hasOpened = false;

	/**
	 * Create the SSE configuration for this manager
	 * Must be implemented by subclasses
	 */
	protected abstract createConfig(): SSEConfig<T>;

	/**
	 * Runs when the stream opens again after having been open, so a tab returning from hidden or
	 * from a dropped connection can refetch what it missed meanwhile.
	 */
	protected onReconnected(): void {}

	/**
	 * Runs when the stream closes for a hidden tab, so state that tracks a live stream can reset.
	 */
	protected onPaused(): void {}

	/** Holds a stream open whenever the tab is visible. Safe to call repeatedly. */
	connect() {
		if (!this.client) {
			const config = this.createConfig();
			this.client = new SSEClient({
				...config,
				onOpen: () => {
					config.onOpen?.();
					if (this.hasOpened) this.onReconnected();
					this.hasOpened = true;
				}
			});
			document.addEventListener('visibilitychange', this.onVisibilityChange);
		}
		if (document.visibilityState === 'visible') this.client.connect();
	}

	disconnect() {
		this.clearHiddenTimer();
		if (this.client) {
			document.removeEventListener('visibilitychange', this.onVisibilityChange);
			this.client.disconnect();
			this.client = null;
		}
		this.hasOpened = false;
	}

	isConnected(): boolean {
		return this.client?.isConnected() ?? false;
	}

	private onVisibilityChange = () => {
		if (!this.client) return;
		if (document.visibilityState === 'hidden') {
			if (this.hiddenTimer) return;
			this.hiddenTimer = setTimeout(() => {
				this.hiddenTimer = null;
				this.client?.disconnect();
				this.onPaused();
			}, HIDDEN_GRACE_MS);
		} else {
			this.clearHiddenTimer();
			this.client.connect();
		}
	};

	private clearHiddenTimer() {
		if (this.hiddenTimer) {
			clearTimeout(this.hiddenTimer);
			this.hiddenTimer = null;
		}
	}
}
