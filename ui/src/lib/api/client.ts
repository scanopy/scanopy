/**
 * Typed API client using openapi-fetch
 *
 * This client provides type-safe API calls based on the OpenAPI spec.
 * It includes middleware for request caching, error handling, and credentials.
 */

import createClient, { type Middleware } from 'openapi-fetch';
import type { paths, components } from './schema';
import { pushError } from '$lib/shared/stores/feedback';
import { translateError, type ApiErrorResponse } from '$lib/i18n/errors';
import type { ErrorCode } from '$lib/generated/error-codes';
import {
	common_httpError,
	common_requestTimedOut,
	common_requestUnconfirmed
} from '$lib/paraglide/messages';
import { env } from '$env/dynamic/public';

// Re-export schema types for convenience
export type { paths, components };

// Common type aliases
export type ApiMeta = {
	api_version: number;
	server_version: string;
};

export type ApiResponse<T> = {
	success: boolean;
	data: T | null;
	error: string | null;
	meta: ApiMeta;
};

export class ApiError extends Error {
	status: number;
	retryAfter: number | null;
	/** The backend's error code, when the failure carried one. */
	code: string | null;
	constructor(
		message: string,
		status: number,
		retryAfter: number | null = null,
		code: string | null = null
	) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.retryAfter = retryAfter;
		this.code = code;
	}
}

/** A request the server did not answer within its budget. */
export class RequestTimeoutError extends Error {
	readonly timeoutMs: number;
	readonly method: string;
	constructor(timeoutMs: number, method: string) {
		super(`Request timed out after ${timeoutMs}ms`);
		this.name = 'RequestTimeoutError';
		this.timeoutMs = timeoutMs;
		this.method = method;
	}
}

/**
 * How long a request may wait for the server. Without a limit, a request the server never
 * answers holds its concurrency slot and leaves whatever is waiting on it, the Scans tab's session
 * list for one, loading for good.
 */
const DEFAULT_TIMEOUT_MS = 30_000;

type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

/**
 * Requests that legitimately take longer than the default, with their own budget. `null` means no
 * limit: the work scales with the size of the network and has no useful ceiling.
 *
 * Keyed by the schema path, so renaming an endpoint breaks the build here rather than quietly
 * dropping its budget.
 */
const TIMEOUT_OVERRIDES: {
	method: HttpMethod;
	path: keyof paths;
	timeoutMs: number | null;
}[] = [
	// The whole network's entities are copied in one transaction inside the request.
	{ method: 'POST', path: '/api/v1/snapshots', timeoutMs: null },
	{ method: 'POST', path: '/api/v1/organizations/{id}/reset', timeoutMs: null },
	{ method: 'POST', path: '/api/v1/organizations/{id}/populate-demo', timeoutMs: null },
	{ method: 'POST', path: '/api/v1/daemons/bulk-delete', timeoutMs: null },
	{ method: 'POST', path: '/api/v1/discovery/bulk-delete', timeoutMs: null },
	{ method: 'POST', path: '/api/v1/hosts/bulk-delete', timeoutMs: null },
	{ method: 'POST', path: '/api/v1/subnets/bulk-delete', timeoutMs: null },
	// These can contact a daemon, retrying for up to about two and a half minutes.
	{ method: 'POST', path: '/api/v1/discovery/start-session', timeoutMs: 180_000 },
	{ method: 'POST', path: '/api/v1/hosts/{id}/rescan', timeoutMs: 180_000 },
	{ method: 'POST', path: '/api/v1/daemons/{id}/retry-connection', timeoutMs: 180_000 },
	{ method: 'POST', path: '/api/v1/discovery/{session_id}/cancel', timeoutMs: 60_000 },
	{ method: 'POST', path: '/api/v1/daemons/test-reachability', timeoutMs: 60_000 },
	{ method: 'POST', path: '/api/v1/daemons/provision', timeoutMs: 120_000 },
	// Sends an email over the organisation's mail server.
	{ method: 'POST', path: '/api/v1/daemons/email-install-command', timeoutMs: 120_000 },
	{ method: 'POST', path: '/api/v1/subnets/{id}/merge', timeoutMs: 120_000 },
	// Graph builds over the whole network.
	{ method: 'GET', path: '/api/v1/topology', timeoutMs: 120_000 },
	{ method: 'GET', path: '/api/v1/topology/data', timeoutMs: 120_000 },
	{ method: 'GET', path: '/api/v1/topology/{id}', timeoutMs: 120_000 }
];

/** Billing calls go to Stripe from inside the request, and a plan change can take a while. */
const BILLING_TIMEOUT_MS = 90_000;

function timeoutFor(method: string, schemaPath: string): number | null {
	const override = TIMEOUT_OVERRIDES.find((o) => o.method === method && o.path === schemaPath);
	if (override) return override.timeoutMs;
	if (schemaPath.startsWith('/api/billing/') || schemaPath.startsWith('/api/v1/licenses/')) {
		return BILLING_TIMEOUT_MS;
	}
	return DEFAULT_TIMEOUT_MS;
}

/** Per-request settings carried on the `Request` into `rateLimitedFetch`. */
type RequestSettings = { timeoutMs?: number | null };

/** Stamps each request with its budget. The timer itself starts in `rateLimitedFetch`. */
const timeoutMiddleware: Middleware = {
	onRequest({ request, schemaPath }) {
		(request as Request & RequestSettings).timeoutMs = timeoutFor(request.method, schemaPath);
		return undefined;
	}
};

// Global rate limit state — when any request gets 429'd, all requests wait
// Persisted to sessionStorage so it survives page refreshes
let rateLimitedUntil = 0;

if (typeof window !== 'undefined') {
	try {
		const stored = sessionStorage.getItem('rateLimitedUntil');
		if (stored) rateLimitedUntil = parseInt(stored, 10) || 0;
	} catch {
		// sessionStorage may not be available
	}
}

const MIN_RATE_LIMIT_MS = 3000; // Minimum wait — Retry-After: 1 is too optimistic for stampedes

// Concurrency control — limits parallel requests so when a 429 arrives,
// queued requests see the gate before going out
const MAX_CONCURRENT = 6;
let activeRequests = 0;
const pendingQueue: Array<() => void> = [];
const STAGGER_MS = 500; // Stagger releases during rate-limit recovery

function setRateLimitedUntil(until: number) {
	rateLimitedUntil = Math.max(rateLimitedUntil, until);
	try {
		sessionStorage.setItem('rateLimitedUntil', String(rateLimitedUntil));
	} catch {
		// sessionStorage may not be available
	}
}

export function isRateLimited(): boolean {
	return Date.now() < rateLimitedUntil;
}

export function getRateLimitDelay(): number {
	return Math.max(0, rateLimitedUntil - Date.now());
}

// Single drain timer — ensures only ONE pending drain at a time,
// preventing multiple acquireSlot/releaseSlot calls from defeating stagger
let drainTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleDrain(delay: number): void {
	if (drainTimer !== null) return; // Already scheduled
	drainTimer = setTimeout(() => {
		drainTimer = null;
		drainQueue();
	}, delay);
}

function acquireSlot(): Promise<void> {
	if (activeRequests < MAX_CONCURRENT && !isRateLimited()) {
		activeRequests++;
		return Promise.resolve();
	}
	return new Promise<void>((resolve) => {
		pendingQueue.push(resolve);
		if (isRateLimited()) {
			scheduleDrain(getRateLimitDelay() + 100);
		}
	});
}

function releaseSlot(): void {
	activeRequests = Math.max(0, activeRequests - 1);
	if (pendingQueue.length === 0) return;

	if (isRateLimited()) {
		scheduleDrain(getRateLimitDelay() + 100);
	} else {
		const recentlyRateLimited = rateLimitedUntil > 0 && Date.now() - rateLimitedUntil < 10000;
		if (recentlyRateLimited) {
			scheduleDrain(STAGGER_MS);
		} else {
			drainQueue();
		}
	}
}

function drainQueue(): void {
	if (pendingQueue.length === 0 || activeRequests >= MAX_CONCURRENT) return;

	if (isRateLimited()) {
		scheduleDrain(getRateLimitDelay() + 100);
		return;
	}

	// Release exactly one request
	activeRequests++;
	const next = pendingQueue.shift()!;
	next();

	// Schedule next release if more pending
	if (pendingQueue.length > 0) {
		const recentlyRateLimited = rateLimitedUntil > 0 && Date.now() - rateLimitedUntil < 10000;
		if (recentlyRateLimited) {
			scheduleDrain(STAGGER_MS);
		} else {
			drainQueue();
		}
	}
}

/**
 * Custom fetch with concurrency control and rate limit detection.
 *
 * Limits in-flight requests to MAX_CONCURRENT. When a 429 arrives,
 * the gate is set BEFORE releasing the slot — so queued requests
 * see the gate and wait instead of stampeding.
 */
async function rateLimitedFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
	await acquireSlot();

	// Armed only now that the request holds a slot: during rate-limit recovery the queue releases
	// one request every half second, and a budget started at call time would be spent waiting.
	const settings: RequestSettings =
		input instanceof Request ? (input as Request & RequestSettings) : {};
	const timeoutMs = settings.timeoutMs === undefined ? DEFAULT_TIMEOUT_MS : settings.timeoutMs;
	const method = input instanceof Request ? input.method : (init?.method ?? 'GET');
	const controller = new AbortController();
	// A caller's own abort still wins.
	const callerSignal = input instanceof Request ? input.signal : (init?.signal ?? undefined);
	const forwardAbort = () => controller.abort(callerSignal?.reason);
	if (callerSignal?.aborted) {
		controller.abort(callerSignal.reason);
	} else {
		callerSignal?.addEventListener('abort', forwardAbort, { once: true });
	}
	let timedOut = false;
	const timer =
		timeoutMs === null
			? undefined
			: setTimeout(() => {
					timedOut = true;
					controller.abort();
				}, timeoutMs);

	try {
		const response = await fetch(input, { ...init, signal: controller.signal });

		if (response.status === 429) {
			const retryAfterHeader = response.headers.get('Retry-After');
			const retryAfter = retryAfterHeader ? parseInt(retryAfterHeader, 10) : null;
			const headerMs = retryAfter && !isNaN(retryAfter) ? retryAfter * 1000 : 5000;
			const retryAfterMs = Math.max(headerMs, MIN_RATE_LIMIT_MS);
			setRateLimitedUntil(Date.now() + retryAfterMs);
		}

		return response;
	} catch (error) {
		if (timedOut && timeoutMs !== null) {
			// The toast is raised here rather than in `errorMiddleware.onError`, which skips this
			// error: only this layer knows which method timed out. For anything but a read, the
			// server may well have done the work and only the answer was lost, and the message says
			// so rather than calling it a failure.
			pushError(method === 'GET' ? common_requestTimedOut() : common_requestUnconfirmed());
			throw new RequestTimeoutError(timeoutMs, method);
		}
		throw error;
	} finally {
		if (timer !== undefined) clearTimeout(timer);
		callerSignal?.removeEventListener('abort', forwardAbort);
		releaseSlot();
	}
}

/**
 * Get the server URL based on environment configuration
 */
export function getServerUrl(): string {
	// If API is on a different host/port, use explicit config
	if (env.PUBLIC_SERVER_HOSTNAME && env.PUBLIC_SERVER_HOSTNAME !== 'default') {
		const protocol = env.PUBLIC_SERVER_PROTOCOL || 'http';
		const port = env.PUBLIC_SERVER_PORT ? `:${env.PUBLIC_SERVER_PORT}` : '';
		return `${protocol}://${env.PUBLIC_SERVER_HOSTNAME}${port}`;
	}

	// Otherwise, use the exact same origin as the browser
	if (typeof window !== 'undefined') {
		return window.location.origin;
	}

	// SSR fallback
	return '';
}

// Request cache for debouncing duplicate requests
interface CacheEntry {
	promise: Promise<Response>;
	timestamp: number;
	completed: boolean;
	result?: Response;
}

const requestCache = new Map<string, CacheEntry>();
const DEBOUNCE_MS = 500; // Increased from 250 to reduce rapid invalidation impact
const MAX_CACHE_SIZE = 50; // Limit cache size to prevent memory growth

function getRequestKey(request: Request): string {
	const method = request.method;
	const url = request.url;
	// For GET requests, just use method + url
	// For mutations, we don't cache (each should execute)
	if (method === 'GET') {
		return `${method}:${url}`;
	}
	return ''; // Don't cache mutations
}

function cleanupExpiredRequests() {
	const now = Date.now();
	// Remove expired entries
	for (const [key, cache] of requestCache.entries()) {
		if (now - cache.timestamp > DEBOUNCE_MS) {
			requestCache.delete(key);
		}
	}
	// If still over limit, remove oldest entries
	if (requestCache.size > MAX_CACHE_SIZE) {
		const entries = Array.from(requestCache.entries());
		entries.sort((a, b) => a[1].timestamp - b[1].timestamp);
		const toRemove = entries.slice(0, entries.length - MAX_CACHE_SIZE);
		for (const [key] of toRemove) {
			requestCache.delete(key);
		}
	}
}

/**
 * Caching middleware - debounces identical GET requests within 250ms
 */
const cachingMiddleware: Middleware = {
	async onRequest({ request }) {
		cleanupExpiredRequests();

		const cacheKey = getRequestKey(request);
		if (!cacheKey) return undefined; // Don't cache mutations

		const cached = requestCache.get(cacheKey);
		if (cached) {
			const timeSinceRequest = Date.now() - cached.timestamp;
			if (timeSinceRequest < DEBOUNCE_MS) {
				if (cached.completed && cached.result) {
					// Return cached response (clone to allow re-reading body)
					return cached.result.clone();
				}
				// Request is still in progress, wait for it
				const result = await cached.promise;
				return result.clone();
			}
		}

		return undefined;
	},
	async onResponse({ request, response }) {
		const cacheKey = getRequestKey(request);
		if (!cacheKey) return response;

		// Store in cache
		const entry = requestCache.get(cacheKey);
		if (entry) {
			entry.completed = true;
			entry.result = response.clone();
		}

		return response;
	}
};

/**
 * Error handling middleware - shows toast notifications on errors
 *
 * Uses translateError to display localized error messages when the backend
 * provides an error code. Falls back to the raw error message or HTTP status.
 */
const BILLING_SELF_HOSTED_PLAN_LOCKED: ErrorCode = 'billing_self_hosted_plan_locked';

const errorMiddleware: Middleware = {
	async onResponse({ response }) {
		if (!response.ok) {
			// Don't show error toasts for 401 (expected when not logged in)
			if (response.status === 401) {
				return response;
			}
			// For 429, throw for TanStack Query retry (gate already set by rateLimitedFetch)
			if (response.status === 429) {
				const retryAfterHeader = response.headers.get('Retry-After');
				const retryAfter = retryAfterHeader ? parseInt(retryAfterHeader, 10) : null;
				throw new ApiError(
					'Rate limited',
					429,
					retryAfter && !isNaN(retryAfter) ? retryAfter : null
				);
			}
			try {
				const errorData: ApiErrorResponse = await response.clone().json();
				// Expected while the org is on a self-hosted plan: the UI already
				// holds the app behind the Settings billing gate, so no toast.
				if (errorData.code === BILLING_SELF_HOSTED_PLAN_LOCKED) {
					return response;
				}
				pushError(translateError(errorData));
			} catch {
				pushError(common_httpError({ status: response.status, statusText: response.statusText }));
			}
		}
		return response;
	},

	// A rejected fetch — dropped connection, DNS failure, CORS — never reaches
	// `onResponse`, so until now it reported nothing anywhere except the handful
	// of callers that toasted for themselves. This layer owns error reporting, so
	// it has to cover that case too.
	//
	// Returning undefined leaves the original error to rethrow, so TanStack still
	// sees the failure. The 429 `throw` above stays inside `onResponse`, outside
	// the block that feeds this hook, so retries are unaffected.
	onError({ error }) {
		// Already reported where the timeout was detected, with wording that depends on the method.
		if (error instanceof RequestTimeoutError) return;
		pushError(
			error instanceof Error && error.message
				? error.message
				: common_httpError({ status: 0, statusText: 'Network error' })
		);
	}
};

/**
 * Create the typed API client
 *
 * Note: baseUrl does NOT include '/api' because the OpenAPI schema paths
 * already include the '/api' prefix (e.g., '/api/hosts', '/api/networks').
 */
export const apiClient = createClient<paths>({
	baseUrl: getServerUrl(),
	credentials: 'include',
	headers: {
		'Content-Type': 'application/json'
	},
	fetch: rateLimitedFetch
});

// Add middleware (rate limiting and timeouts handled by custom fetch wrapper above)
apiClient.use(timeoutMiddleware);
apiClient.use(cachingMiddleware);
apiClient.use(errorMiddleware);

export default apiClient;
