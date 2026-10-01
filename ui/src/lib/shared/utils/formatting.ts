import type { Port } from '$lib/features/hosts/types/base';
import { displaySettings } from '$lib/shared/stores/display-settings.svelte';
import { formatDateTime, formatRelative } from './date-format';

/**
 * Lowercase a string while preserving runs of 2+ consecutive uppercase letters
 * as acronyms. Examples:
 *   "Host"           → "host"
 *   "IP Address"     → "IP address"
 *   "IP Addresses"   → "IP addresses"
 *   "Daemon API Key" → "daemon API key"
 *   "VLAN"           → "VLAN"
 */
export function lowercasePreservingAcronyms(s: string): string {
	return s.replace(/(\p{Lu}{2,})|./gu, (m, acronym) => (acronym ? acronym : m.toLowerCase()));
}

export const uuidv4Sentinel: string = '00000000-0000-0000-0000-000000000000';

export const utcTimeZoneSentinel: string = '1970-01-01T00:00:00Z';

export function formatDuration(startTime: string, endTime?: string) {
	if (!startTime) return '';

	const start = new Date(startTime);
	const end = endTime ? new Date(endTime) : new Date();
	const durationMs = end.getTime() - start.getTime();

	const totalSeconds = Math.floor(durationMs / 1000);
	const hours = Math.floor(totalSeconds / 3600);
	const minutes = Math.floor((totalSeconds % 3600) / 60);
	const seconds = totalSeconds % 60;

	// Format with leading zeros
	const hh = hours.toString().padStart(2, '0');
	const mm = minutes.toString().padStart(2, '0');
	const ss = seconds.toString().padStart(2, '0');

	return `${hh}:${mm}:${ss}`;
}

export function formatDurationHuman(totalSeconds: number): string {
	const weeks = Math.floor(totalSeconds / 604800);
	const days = Math.floor((totalSeconds % 604800) / 86400);
	const hours = Math.floor((totalSeconds % 86400) / 3600);
	const minutes = Math.round((totalSeconds % 3600) / 60);

	const parts: string[] = [];
	if (weeks > 0) parts.push(`${weeks} week${weeks !== 1 ? 's' : ''}`);
	if (days > 0) parts.push(`${days} day${days !== 1 ? 's' : ''}`);
	if (hours > 0) parts.push(`${hours} hour${hours !== 1 ? 's' : ''}`);
	if (parts.length === 0 || (weeks === 0 && days === 0 && hours === 0)) {
		if (minutes === 0) {
			parts.push('< 1 minute');
		} else {
			parts.push(`${minutes} minute${minutes !== 1 ? 's' : ''}`);
		}
	}

	return parts.join(', ');
}

// Date helpers. Each one is a style preset over `formatDateTime`, which applies the user's
// display settings (date order, clock, time zone), so every displayed date follows them.

/** Date and time, e.g. `Jun 21, 2026, 03:45 PM`. */
export function formatTimestamp(timestamp: string | Date): string {
	return formatDateTime(timestamp, { date: 'short', time: true }, displaySettings.current);
}

/** Date only (no time), e.g. `Jun 21, 2026`. */
export function formatDate(timestamp: string | Date): string {
	return formatDateTime(timestamp, { date: 'short' }, displaySettings.current);
}

/**
 * Long date, e.g. `October 8, 2026`. Pass `timeZone: 'UTC'` for a date-only string
 * (`2026-10-08`), which `Date` parses as UTC midnight and would otherwise show a day early west
 * of UTC.
 */
export function formatLongDate(value: string | Date, timeZone?: string): string {
	return formatDateTime(value, { date: 'long', timeZone }, displaySettings.current);
}

// Truncate ID for display (show first 8 characters + ellipsis if longer than 12)
export function formatId(id: string): string {
	if (id.length <= 12) {
		return id;
	}
	return `${id.substring(0, 8)}...`;
}
/** How long ago, e.g. `5m ago`, or the full timestamp when the user turned relative times off. */
export function formatRelativeTime(timestamp: string | Date): string {
	return formatRelative(timestamp, displaySettings.current);
}

export function formatPort(port: Port): string {
	return `${port.number}${port.protocol == 'Tcp' ? '/tcp' : '/udp'}`;
}
