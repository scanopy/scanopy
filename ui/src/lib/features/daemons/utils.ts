import { fieldDefs } from './config';
import type { Daemon } from './types/base';
import type { FormValue } from '$lib/shared/components/forms/validators';
import type { TagProps } from '$lib/shared/components/data/types';
import { toColor } from '$lib/shared/utils/styling';
import { CircleHelp } from 'lucide-svelte';

export const DAEMON_STATUS_DOCS_URL = 'https://scanopy.net/docs/daemon-status';

/**
 * Returns the highest-priority status tag for a daemon, or null if healthy.
 * Priority: Unreachable > Standby > Deprecated > Outdated
 */
export function getDaemonStatusTag(daemon: Daemon): TagProps | null {
	const docsTag = { href: DAEMON_STATUS_DOCS_URL, icon: CircleHelp };

	if (daemon.is_unreachable === true) {
		return { label: 'Unreachable', color: toColor('red'), ...docsTag };
	}
	if (daemon.standby === true) {
		return { label: 'Standby', color: toColor('purple'), ...docsTag };
	}
	switch (daemon.version_status.status) {
		case 'Deprecated':
			return { label: 'Deprecated', color: toColor('orange'), ...docsTag };
		case 'Outdated':
			return { label: 'Outdated', color: toColor('yellow'), ...docsTag };
		default:
			return null;
	}
}

export type DaemonOS = 'linux' | 'macos' | 'windows' | 'freebsd' | 'openbsd';

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
	// UI state fields (not part of daemon config, just for form interaction)
	defaults.keySource = 'generate';
	defaults.existingKeyInput = '';
	return defaults;
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
	let cmd = `${prefix}${binary} --server-url ${serverUrl}`;

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
			if (value) cmd += ` ${def.cliFlag} true`;
		} else {
			cmd += ` ${def.cliFlag} ${value}`;
		}
	}

	return cmd;
}

export function buildDockerCompose(
	serverUrl: string,
	networkId: string,
	key: string,
	values: Record<string, string | number | boolean>,
	userId: string | null
): string {
	const envVars: string[] = [`SCANOPY_SERVER_URL=${serverUrl}`, `SCANOPY_DAEMON_API_KEY=${key}`];

	if (networkId) {
		envVars.splice(1, 0, `SCANOPY_NETWORK_ID=${networkId}`);
	}

	// Include user_id for new daemon registrations
	if (userId) {
		envVars.push(`SCANOPY_USER_ID=${userId}`);
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

		if (def.type === 'boolean') {
			if (value) envVars.push(`${def.envVar}=true`);
		} else {
			envVars.push(`${def.envVar}=${value}`);
		}
	}

	const dockerProxyDef = fieldDefs.find((d) => d.id === 'dockerProxy');
	const hasDockerProxy =
		values.dockerProxy &&
		values.dockerProxy !== '' &&
		(!dockerProxyDef || fieldPassesValidation(dockerProxyDef, values.dockerProxy));
	const volumeMounts = ['daemon-config:/root/.config/daemon'];
	if (!hasDockerProxy) {
		volumeMounts.push('/var/run/docker.sock:/var/run/docker.sock:ro');
	}

	const lines = [
		'services:',
		'  daemon:',
		'    image: ghcr.io/scanopy/scanopy/daemon:latest',
		'    container_name: scanopy-daemon',
		'    network_mode: host',
		'    privileged: true',
		'    restart: unless-stopped',
		'    environment:',
		...envVars.map((v) => `      - ${v}`),
		'    volumes:',
		...volumeMounts.map((v) => `      - ${v}`),
		'',
		'volumes:',
		'  daemon-config:'
	];

	return lines.join('\n');
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
