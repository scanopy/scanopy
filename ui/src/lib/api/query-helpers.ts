/**
 * Helpers for turning an openapi-fetch result into a value or a thrown error.
 *
 * openapi-fetch resolves every call to `{ data, error, response }`. On a
 * failure `data` is undefined and the backend's error body is in `error`, so a
 * check like `throw new Error(data?.error || 'fallback')` can never read the
 * server's message: `data?.error` is always undefined there, and the hardcoded
 * fallback always wins. These helpers read the right half.
 *
 * They decide on the HTTP status. The backend sends every failure as an
 * `ApiError` with a real error status, and a successful envelope can only be
 * built with `success: true`, so `response.ok` is the whole signal.
 *
 * Reporting is not their job: the client's error middleware has already toasted
 * a failed response by the time one of these throws. The thrown `ApiError`
 * carries the translated message, the status and the backend's code for any
 * caller that needs to branch on them.
 */

import { ApiError } from './client';
import { translateError } from '$lib/i18n/errors';

/** An openapi-fetch result, reduced to what these helpers read. */
interface FetchResult<E> {
	data?: E;
	error?: unknown;
	response: Response;
}

type Envelope = { data?: unknown };
type WithData<E extends Envelope> = E & { data: NonNullable<E['data']> };

function asRecord(value: unknown): Record<string, unknown> | undefined {
	return typeof value === 'object' && value !== null
		? (value as Record<string, unknown>)
		: undefined;
}

/** Build the thrown error from a failed response's body. */
function failure(error: unknown, response: Response): ApiError {
	const body = asRecord(error);
	const code = typeof body?.code === 'string' ? body.code : undefined;
	const serverMessage = typeof body?.error === 'string' ? body.error : undefined;
	const message =
		code || serverMessage
			? translateError({ code, error: serverMessage, params: asRecord(body?.params) })
			: `${response.status} ${response.statusText}`;
	return new ApiError(message, response.status, null, code ?? null);
}

/** For endpoints that return no payload: throws unless the request succeeded. */
export function requireSuccess(result: FetchResult<unknown>): void {
	if (!result.response.ok) {
		throw failure(result.error, result.response);
	}
}

/**
 * The whole envelope, for the callers that need more than the payload, such as
 * a paginated list reading `meta.pagination`. Throws on failure, and on a
 * successful response that carries no payload.
 */
export function unwrapEnvelope<E extends Envelope>(result: FetchResult<E>): WithData<E> {
	requireSuccess(result);
	const envelope = result.data;
	if (envelope?.data === undefined || envelope.data === null) {
		throw new ApiError(`${result.response.url} returned no data`, result.response.status);
	}
	// Checked just above; TypeScript cannot narrow a property of a generic.
	return envelope as WithData<E>;
}

/** The payload of a successful response. Throws on failure or an empty payload. */
export function unwrapData<E extends Envelope>(result: FetchResult<E>): NonNullable<E['data']> {
	return unwrapEnvelope(result).data;
}
