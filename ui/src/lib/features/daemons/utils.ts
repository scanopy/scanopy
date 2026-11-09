import { coerce, gt } from 'semver';
import { fieldDefs } from './config';
import type { components } from '$lib/api/schema';
import daemonOsMetadata from '$lib/data/daemon-os.json';
import osFamilies from '$lib/data/os-families.json';
import type { Daemon } from './types/base';
import type { FormValue } from '$lib/shared/components/forms/validators';
import { getServerUrl } from '$lib/api/client';
import {
	common_freebsd,
	common_linux,
	common_macos,
	common_windows
} from '$lib/paraglide/messages';

export const DAEMON_STATUS_DOCS_URL = 'https://scanopy.net/docs/reference/daemon-status/';

/**
 * Whether a daemon is known to predate unified discovery: it has checked in, and the version it
 * reported is below the floor.
 *
 * The server answers `supports_unified_discovery` from the version alone, and a daemon awaiting
 * its first connection has none, so asking that flag directly reads every new daemon as legacy —
 * which is what put a "your discoveries will be consolidated" banner in front of daemons that
 * have never run one. No version yet is not the same as an old version.
 */
export function isPreUnifiedDaemon(daemon: Daemon): boolean {
	return !!daemon.last_seen && daemon.version_status?.supports_unified_discovery === false;
}

/// Derived from the generated schema rather than hand-listed: this was a parallel string union
/// duplicating the backend enum, which is what let the email-install-command call pass a display
/// label where an OS identifier was expected.
export type DaemonOS = components['schemas']['DaemonOs'];

/** Lowercase alphanumerics, every other run of characters collapsed to one `separator`. */
export function slugifySiteName(name: string, separator: '-' | '_' = '-'): string {
	return name
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, separator)
		.replace(new RegExp(`^\\${separator}+|\\${separator}+$`, 'g'), '');
}

export function detectOS(): DaemonOS {
	if (typeof navigator === 'undefined') return 'linux';
	const ua = navigator.userAgent.toLowerCase();
	if (ua.includes('win')) return 'windows';
	if (ua.includes('mac')) return 'macos';
	return 'linux';
}

/**
 * Check if a field value passes all its validators
 */
export function fieldPassesValidation(def: (typeof fieldDefs)[0], value: FormValue): boolean {
	if (!def.validators || def.validators.length === 0) return true;
	for (const validator of def.validators) {
		const error = validator(value);
		if (error) return false;
	}
	return true;
}

/**
 * Build default form values from field definitions
 */
export function buildDefaultValues(
	initialName?: string
): Record<string, string | number | boolean> {
	const defaults: Record<string, string | number | boolean> = {};
	for (const def of fieldDefs) {
		if (def.id === 'name' && initialName) {
			defaults[def.id] = initialName;
		} else {
			defaults[def.id] = def.defaultValue ?? '';
		}
	}
	return defaults;
}

/**
 * The client-settable advanced daemon settings, keyed by their daemon config-field names. Fed to
 * the install-command builder, which folds them into the emitted command + MSI filename.
 */
export interface AdvancedInstallConfig {
	log_level?: string;
	log_file?: string;
	heartbeat_interval?: number;
	bind_address?: string;
	allow_self_signed_certs?: boolean;
	accept_invalid_scan_certs?: boolean;
	interfaces?: string[];
}

/**
 * Collect the advanced daemon settings from the wizard form.
 *
 * Only advanced fields (those with a `section`) are included — everything else the server owns
 * and derives from the daemon record. Values equal to their default are skipped, so the command
 * carries only what the user changed and the whole MSI config fits inside a 255-character
 * filename.
 */
export function buildInstallConfig(
	values: Record<string, string | number | boolean>
): AdvancedInstallConfig {
	const config: Record<string, string | number | boolean | string[]> = {};

	for (const def of fieldDefs) {
		if (!def.section || def.docsOnly) continue;

		const value = values[def.id];
		if (value === '' || value === null || value === undefined) continue;
		if (value === def.defaultValue) continue;
		if (!fieldPassesValidation(def, value)) continue;

		// `--interfaces` is comma-delimited on the CLI but a list here.
		if (def.id === 'interfaces') {
			const list = String(value)
				.split(',')
				.map((s) => s.trim())
				.filter(Boolean);
			if (list.length > 0) config.interfaces = list;
			continue;
		}

		config[camelToSnake(def.id)] = value;
	}

	return config as AdvancedInstallConfig;
}

function camelToSnake(id: string): string {
	return id.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
}

/**
 * Construct full daemon URL from base URL and port.
 */
export function constructDaemonUrl(baseUrl: string, port: number): string {
	try {
		const parsed = new globalThis.URL(baseUrl);
		const protocol = parsed.protocol;
		const hostname = parsed.hostname;
		const pathname = parsed.pathname === '/' ? '' : parsed.pathname;
		return `${protocol}//${hostname}:${port}${pathname}`;
	} catch {
		return `${baseUrl}:${port}`;
	}
}

const WINDOWS_MSI_GITHUB_URL =
	'https://github.com/scanopy/scanopy/releases/latest/download/scanopy-daemon-windows-amd64.msi';

/**
 * Download the Windows daemon MSI, saved under the given (per-daemon) filename.
 *
 * GitHub's release-asset URL redirects to a signed Azure Blob URL that bakes in its own fixed
 * `Content-Disposition`, which browsers always honor over an `<a download>` attribute — so a
 * direct link can never rename the file. Instead this fetches the MSI through our own origin
 * (which proxies it from GitHub) and saves it as a blob under the caller's filename.
 *
 * Fails open: if the proxy is unavailable (e.g. a self-hosted server with no outbound internet
 * access), falls back to opening the direct GitHub link, where the browser saves it under
 * GitHub's fixed name. Returns `true` when the file was saved under `filename` (no rename
 * needed), `false` when it fell back to the direct link (caller should show a rename hint).
 */
export async function downloadDaemonMsi(filename: string): Promise<boolean> {
	try {
		const url = new globalThis.URL('/api/install/windows-msi', getServerUrl());
		const response = await fetch(url.toString());
		if (!response.ok) throw new Error(response.statusText);

		const blob = await response.blob();
		const blobUrl = globalThis.URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.href = blobUrl;
		link.download = filename;
		document.body.appendChild(link);
		link.click();
		document.body.removeChild(link);
		globalThis.URL.revokeObjectURL(blobUrl);
		return true;
	} catch {
		window.open(WINDOWS_MSI_GITHUB_URL, '_blank');
		return false;
	}
}

type OsFamily = components['schemas']['OsFamily'];

function isOsFamily(value: string): value is OsFamily {
	return osFamilies.some((family) => family.id === value);
}

/**
 * The credential-path OS family for a daemon OS, from the backend's `From<DaemonOs> for OsFamily`
 * (`daemon-os.json`): Windows paths for a Windows daemon, Unix paths for every other.
 */
export function osFamilyOf(os: DaemonOS): OsFamily {
	const family = daemonOsMetadata.find((entry) => entry.id === os)?.metadata.os_family;
	return family && isOsFamily(family) ? family : 'Unix';
}

/** A daemon OS's display name ("macOS", "FreeBSD"), the one label every surface uses. */
export function osLabel(os: DaemonOS): string {
	switch (os) {
		case 'linux':
			return common_linux();
		case 'macos':
			return common_macos();
		case 'windows':
			return common_windows();
		case 'freebsd':
			return common_freebsd();
	}
}

/** Every OS a daemon can report. Fixture ids are `DaemonOs` names, emitted from that enum, so
 *  each is a valid `DaemonOS` by construction. */
export const daemonOsIds: DaemonOS[] = daemonOsMetadata.map((entry) => entry.id as DaemonOS);

/**
 * Whether a daemon at `daemonVersion` is at or below the release `lastWithout`, the last one
 * without some capability (the server's `last_without_*` gates).
 *
 * `coerce` tolerates partial/odd version strings; a missing daemon version or a missing gate means
 * there is nothing to gate on (returns false).
 */
export function daemonVersionAtOrBelow(
	lastWithout: string | undefined | null,
	daemonVersion: string | undefined | null
): boolean {
	if (!lastWithout || !daemonVersion) return false;
	const dv = coerce(daemonVersion);
	return dv ? !gt(dv, lastWithout) : false;
}

/**
 * The command that prints one run's log lines on its daemon's host, from the daemon OS's
 * `log_command` in `daemon-os.json` with the run's session id swapped in.
 */
export function runLogCommand(os: DaemonOS, sessionId: string): string | null {
	const metadata = daemonOsMetadata.find((entry) => entry.id === os)?.metadata;
	if (!metadata) return null;
	return metadata.log_command.replaceAll(metadata.log_command_session_placeholder, sessionId);
}
