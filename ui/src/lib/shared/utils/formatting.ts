import type { Port } from '$lib/features/hosts/types/base';

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

export function formatTimestamp(timestamp: string): string {
	try {
		const date = new Date(timestamp);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit',
			hour12: false
		});
	} catch {
		return timestamp; // Fallback to raw string if parsing fails
	}
}

/** Date only (no time), e.g. "Jun 21, 2026". */
export function formatDate(timestamp: string): string {
	try {
		return new Date(timestamp).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	} catch {
		return timestamp;
	}
}

/**
 * Long date in the viewer's locale, e.g. `October 8, 2026`. Pass
 * `timeZone: 'UTC'` for a date-only string (`2026-10-08`), which `Date`
 * parses as UTC midnight and would otherwise show a day early west of UTC.
 */
export function formatLongDate(value: string | Date, timeZone?: string): string {
	return new Date(value).toLocaleDateString(undefined, {
		month: 'long',
		day: 'numeric',
		year: 'numeric',
		timeZone
	});
}

/**
 * Compact numeric date, e.g. `8/3/26`.
 *
 * For dense lists where a date is one column among many and "Aug 3, 2026" or a
 * full timestamp costs more width than the extra precision is worth.
 */
export function formatDateNumeric(timestamp: string | Date): string {
	const date = timestamp instanceof Date ? timestamp : new Date(timestamp);
	if (Number.isNaN(date.getTime())) return String(timestamp);

	return date.toLocaleDateString(undefined, {
		year: '2-digit',
		month: 'numeric',
		day: 'numeric'
	});
}

// Truncate ID for display (show first 8 characters + ellipsis if longer than 12)
export function formatId(id: string): string {
	if (id.length <= 12) {
		return id;
	}
	return `${id.substring(0, 8)}...`;
}
export function formatRelativeTime(timestamp: string): string {
	const now = Date.now();
	const then = new Date(timestamp).getTime();
	const diff = Math.max(0, now - then);
	const minutes = Math.floor(diff / 60000);
	if (minutes < 1) return 'just now';
	if (minutes < 60) return `${minutes}m ago`;
	const hours = Math.floor(minutes / 60);
	if (hours < 24) return `${hours}h ago`;
	const days = Math.floor(hours / 24);
	return `${days}d ago`;
}

export function formatPort(port: Port): string {
	return `${port.number}${port.protocol == 'Tcp' ? '/tcp' : '/udp'}`;
}
