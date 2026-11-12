/**
 * Error translation utilities for API errors.
 *
 * This module provides type-safe translation of backend error codes
 * using Paraglide JS. When an error code is present, it looks up
 * the corresponding translation key (errors_<code>) and interpolates
 * any parameters. Falls back to the server's error message if no
 * translation is available.
 */

import { type ErrorCode, ERROR_CODES } from '#lib/generated/error-codes.ts';
import * as m from '#lib/paraglide/messages.js';

/**
 * API error response structure from the backend.
 * The success field is optional since we only need code/params/error for translation.
 */
export interface ApiErrorResponse {
	success?: boolean;
	error?: string;
	code?: string;
	params?: Record<string, unknown>;
}

/**
 * Type guard to check if a string is a valid ErrorCode
 */
function isErrorCode(code: string): code is ErrorCode {
	return code in ERROR_CODES;
}

/**
 * Translate an API error response to a user-friendly message.
 *
 * The function attempts to:
 * 1. Look up the translation key `errors_<code>` in Paraglide messages
 * 2. Interpolate any parameters from the error response
 * 3. Fall back to the server's error message if no translation exists
 *
 * @param error - The API error response containing code, params, and error fields
 * @returns A translated error message string
 *
 * @example
 * ```ts
 * // Backend sends: { code: "entity_not_found", params: { entity: "Host", id: "abc" } }
 * const message = translateError(error);
 * // Returns: "Host with ID 'abc' not found" (translated)
 * ```
 */
export function translateError(error: ApiErrorResponse): string {
	if (error.code && isErrorCode(error.code)) {
		const key = `errors_${error.code}` as keyof typeof m;
		const messageFn = m[key] as ((params: Record<string, unknown>) => string) | undefined;

		if (typeof messageFn === 'function') {
			try {
				return messageFn(error.params ?? {});
			} catch {
				// If interpolation fails, fall through to fallback
			}
		}
	}

	// Fallback to server message
	return error.error ?? 'An error occurred';
}

/**
 * Get the translated error message for a specific error code.
 *
 * Unlike translateError, this function takes the code and params directly
 * rather than an ApiErrorResponse object.
 *
 * @param code - The error code string
 * @param params - Optional parameters for message interpolation
 * @returns A translated error message string
 */
export function getErrorMessage(code: string, params?: Record<string, unknown>): string {
	return translateError({ code, params });
}
