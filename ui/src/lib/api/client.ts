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
	constructor(message: string, status: number, retryAfter: number | null = null) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.retryAfter = retryAfter;
	}
}

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
	try {
		const response = await fetch(input, init);

		if (response.status === 429) {
			const retryAfterHeader = response.headers.get('Retry-After');
			const retryAfter = retryAfterHeader ? parseInt(retryAfterHeader, 10) : null;
			const headerMs = retryAfter && !isNaN(retryAfter) ? retryAfter * 1000 : 5000;
			const retryAfterMs = Math.max(headerMs, MIN_RATE_LIMIT_MS);
			setRateLimitedUntil(Date.now() + retryAfterMs);
		}

		return response;
	} finally {
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
const errorMiddleware: Middleware = {
	async onResponse({ response, options }) {
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
				const errorMsg = translateError(errorData);
				// Only show error if not silenced
				if (!(options as { silenceErrors?: boolean }).silenceErrors) {
					pushError(errorMsg);
				}
			} catch {
				if (!(options as { silenceErrors?: boolean }).silenceErrors) {
					pushError(`HTTP ${response.status}: ${response.statusText}`);
				}
			}
		}
		return response;
	}
};

/**
 * n9e 嵌入:API 路径前缀(运行时注入)。
 * app.html inline 脚本从 iframe 的 ?apiBase= 读入 window.__SCANOPY_API_BASE__。
 * 原生部署无该参数 → 空串 → 行为不变(打 <origin>/api/v1/*)。
 * 嵌入 n9e 时 = '/topo-studio/scanopy-api' → 打 <origin>/topo-studio/scanopy-api/api/v1/*。
 * 导出供绕过 apiClient 的裸 URL(SSE 等)复用。
 */
export const API_BASE_PATH: string =
	(typeof window !== 'undefined' &&
		(window as unknown as { __SCANOPY_API_BASE__?: string }).__SCANOPY_API_BASE__) ||
	'';

/**
 * Create the typed API client
 *
 * Note: baseUrl does NOT include '/api' because the OpenAPI schema paths
 * already include the '/api' prefix (e.g., '/api/hosts', '/api/networks').
 */
export const apiClient = createClient<paths>({
	baseUrl: getServerUrl() + API_BASE_PATH,
	credentials: 'include',
	headers: {
		'Content-Type': 'application/json'
	},
	fetch: rateLimitedFetch
});

// Add middleware (rate limiting handled by custom fetch wrapper above)
apiClient.use(cachingMiddleware);
apiClient.use(errorMiddleware);

export default apiClient;
