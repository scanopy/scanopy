/**
 * A typed client for the public share endpoints, with no middleware.
 *
 * These endpoints serve anonymous viewers on `/share/[id]`, where every failure
 * is rendered inline by the page — a wrong password belongs under the password
 * field, not in a toast. So this client shares the generated types with
 * `apiClient` and nothing else. Each difference below is deliberate:
 *
 * - **No error middleware.** A mistyped password, an expired link and a
 *   disabled share are all expected outcomes here, and `ShareView` shows each
 *   one in place. Toasting them would duplicate the message and leak backend
 *   phrasing to a stranger.
 * - **Relative `baseUrl`.** The share page ships its own CSP with
 *   `connect-src 'self'` (`shares/handlers.rs`), so an absolute cross-origin
 *   API URL is blocked outright on a split-host deployment.
 * - **`credentials: 'omit'`.** The public routes sit behind a wildcard CORS
 *   layer with no `allow_credentials` (`bin/server.rs`), and browsers reject a
 *   credentialed request against `Access-Control-Allow-Origin: *`. There is no
 *   session to send anyway.
 * - **No rate-limit gate or caching.** Those are app-wide state. The share
 *   password endpoint is rate-limited server-side per share, and letting a
 *   viewer's typos trip the global gate would stall the page's own loads.
 *
 * Callers return result objects rather than throwing, so the helpers in
 * `query-helpers.ts` (which throw) are deliberately not used here.
 */

import createClient from 'openapi-fetch';
import type { paths } from './schema';

export const publicApiClient = createClient<paths>({
	baseUrl: '',
	credentials: 'omit',
	headers: {
		'Content-Type': 'application/json'
	}
});

/** The shape every public share function returns: a result, never a throw. */
export interface PublicResult<T> {
	success: boolean;
	data?: T;
	error?: string;
	code?: string;
}

/**
 * Turn openapi-fetch's error half into a message and a code.
 *
 * `translateError` is imported lazily by the callers that need it; here we only
 * read the body, so a non-JSON response (a proxy's HTML error page) yields
 * `undefined` rather than throwing, which the raw `response.json()` used to do.
 */
export function publicFailure(error: unknown, fallback: string): PublicResult<never> {
	const body =
		typeof error === 'object' && error !== null ? (error as Record<string, unknown>) : {};
	const code = typeof body.code === 'string' ? body.code : undefined;
	const message = typeof body.error === 'string' ? body.error : undefined;
	return { success: false, error: message ?? fallback, code };
}
