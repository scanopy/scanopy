import { fieldDefs } from './config';
import type { components } from '$lib/api/schema';
import type { Daemon } from './types/base';
import type { FormValue } from '$lib/shared/components/forms/validators';
import type { TagProps } from '$lib/shared/components/data/types';
import { toColor } from '$lib/shared/utils/styling';
import { getServerUrl } from '$lib/api/client';
import { CircleHelp } from 'lucide-svelte';
import {
	common_deprecated,
	common_healthy,
	common_outdated,
	common_standby,
	common_unreachable,
	common_unsupported,
	common_unknown,
	daemons_awaitingConnection
} from '$lib/paraglide/messages';

export const DAEMON_STATUS_DOCS_URL = 'https://scanopy.net/docs/reference/daemon-status/';

/**
 * Returns the highest-priority status tag for a daemon.
 * Priority: Unsupported > Unreachable > Standby > Deprecated > Outdated > Unknown > Healthy
 *
 * Unsupported ranks above Unreachable: a rejected daemon can't connect, so its
 * unreachability is a symptom — the version is the actionable cause.
 */
/**
 * The `label` is guaranteed, unlike the optional one on `TagProps`: callers use it as the daemon's
 * searchable, groupable status value and as the chip's id, not only as display text.
 */
export function getDaemonStatusTag(daemon: Daemon): TagProps & { label: string } {
	const docsTag = { href: DAEMON_STATUS_DOCS_URL, icon: CircleHelp };

	if (daemon.version_status.status === 'Unsupported') {
		return { label: common_unsupported(), color: toColor('red'), ...docsTag };
	}
	if (daemon.is_unreachable === true) {
		return { label: common_unreachable(), color: toColor('red'), ...docsTag };
	}
	if (daemon.standby === true) {
		return { label: common_standby(), color: toColor('purple'), ...docsTag };
	}
	if (!daemon.last_seen) {
		return { label: daemons_awaitingConnection(), color: toColor('blue'), ...docsTag };
	}
	switch (daemon.version_status.status) {
		case 'Deprecated':
			return { label: common_deprecated(), color: toColor('orange'), ...docsTag };
		case 'Outdated':
			return { label: common_outdated(), color: toColor('yellow'), ...docsTag };
		case 'Unknown':
			return { label: common_unknown(), color: toColor('gray'), ...docsTag };
		default:
			return { label: common_healthy(), color: toColor('green') };
	}
}

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

/**
 * Whether a daemon has an active or upcoming sunset the user should act on.
 * True for Deprecated (a sunset date is scheduled) and Unsupported (past it).
 */
export function hasSunsetWarning(daemon: Daemon): boolean {
	const status = daemon.version_status.status;
	return status === 'Deprecated' || status === 'Unsupported';
}

/// Derived from the generated schema rather than hand-listed: this was a parallel string union
/// duplicating the backend enum, which is what let the email-install-command call pass a display
/// label where an OS identifier was expected.
export type DaemonOS = components['schemas']['DaemonOs'];

export function slugifyNetworkName(name: string): string {
	return name
		.toLowerCase()
		.replace(/[^a-z0-9-]/g, '-')
		.replace(/-+/g, '-')
		.replace(/^-|-$/g, '');
}

export function detectOS(): DaemonOS {
	if (typeof navigator === 'undefined') return 'linux';
	const ua = navigator.userAgent.toLowerCase();
	if (ua.includes('win')) return 'windows';
	if (ua.includes('mac')) return 'macos';
	return 'linux';
}

const DEFAULT_DAEMON_NAME = 'scanopy-daemon';

/**
 * Service/unit identifier the `scanopy-daemon install` engine registers for a given daemon name
 * (systemd unit, Windows SCM service, FreeBSD rc.d). Mirrors `service_id()` in
 * backend/src/daemon/install/mod.rs: the default name keeps its bare id; custom names are
 * namespaced under the `scanopy-daemon-` prefix.
 */
export function daemonServiceId(name: string): string {
	return name === DEFAULT_DAEMON_NAME ? DEFAULT_DAEMON_NAME : `scanopy-daemon-${name}`;
}

/**
 * launchd label the installer uses on macOS. Mirrors `label()` in
 * backend/src/daemon/install/macos.rs.
 */
export function daemonLaunchdLabel(name: string): string {
	return name === DEFAULT_DAEMON_NAME ? 'com.scanopy.daemon' : `com.scanopy.daemon.${name}`;
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
 * and derives from the daemon record. Values equal to their default are skipped, which matches
 * `buildRunCommand` and matters more here: the whole MSI config has to fit inside a 255-character
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

export function buildRunCommand(
	serverUrl: string,
	networkId: string,
	key: string | null,
	values: Record<string, string | number | boolean>,
	daemon: Daemon | null,
	userId: string | null,
	os: DaemonOS = 'linux'
): string {
	const isWindows = os === 'windows';
	const binary = isWindows ? '.\\scanopy-daemon-windows-amd64.exe' : 'scanopy-daemon';
	const prefix = isWindows ? '' : 'sudo ';
	// The `install` subcommand takes the same flags but also registers a system service and
	// writes config.json, so the daemon starts on boot instead of running in the foreground.
	let cmd = `${prefix}${binary} install --server-url ${serverUrl}`;

	if (!daemon && networkId) {
		cmd += ` --network-id ${networkId}`;
	}

	if (key) {
		cmd += ` --daemon-api-key ${key}`;
	}

	// Include user_id for new daemon registrations
	if (!daemon && userId) {
		cmd += ` --user-id ${userId}`;
	}

	const mode = values['mode'] as string;

	for (const def of fieldDefs) {
		const value = values[def.id];

		if (def.docsOnly) {
			continue;
		}

		// Skip daemonUrl - only used for provisioning, not in daemon config
		if (def.id === 'daemonUrl') {
			continue;
		}

		// Skip daemonPort for DaemonPoll mode (server never connects to daemon)
		if (def.id === 'daemonPort' && mode === 'daemon_poll') {
			continue;
		}

		if (value === '' || value === null || value === undefined) {
			continue;
		}

		// Skip fields that don't pass validation
		if (!fieldPassesValidation(def, value)) {
			continue;
		}

		// Skip advanced fields (those with a section) that match their default value
		if (def.section && value === def.defaultValue) {
			continue;
		}

		if (def.id === 'mode') {
			cmd += ` ${def.cliFlag} ${String(value).toLowerCase()}`;
		} else if (def.type === 'boolean') {
			cmd += ` ${def.cliFlag} ${value}`;
		} else {
			cmd += ` ${def.cliFlag} ${value}`;
		}
	}

	// Integration targeting is not carried in the command: it's seeded onto the daemon's
	// discovery row at provision (`seed_credential_refs`) and applied server-side every scan.

	return cmd;
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
