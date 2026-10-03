import type { components } from '$lib/api/schema';

// Re-export generated types
// DaemonResponse includes computed version_status from the API
export type Daemon = components['schemas']['DaemonResponse'];
export type DaemonBase = components['schemas']['DaemonBase'];
export type DaemonMode = components['schemas']['DaemonMode'];

// Version-related types
export type DaemonVersionStatus = components['schemas']['DaemonVersionStatus'];
export type VersionHealthStatus = components['schemas']['VersionHealthStatus'];
export type DeprecationWarning = components['schemas']['DeprecationWarning'];
export type DeprecationSeverity = components['schemas']['DeprecationSeverity'];

// Provisioning types (for ServerPoll mode)
export type ProvisionDaemonRequest = components['schemas']['ProvisionDaemonRequest'];
export type ProvisionDaemonResponse = components['schemas']['ProvisionDaemonResponse'];

// Install-command builder types
export type InstallArtifacts = components['schemas']['InstallArtifacts'];
export type InstallCommandType = components['schemas']['InstallCommandType'];

/** The OS install methods whose content is a ready-to-paste command string. */
export type OsInstallMethod = 'linux' | 'macos' | 'windows' | 'freebsd';

/** The binary install command for an OS. */
export function osInstallCommand(artifacts: InstallArtifacts, os: OsInstallMethod): string {
	return artifacts[os];
}

/**
 * Stand-in the server emits for the daemon api key (it never mints, so it doesn't know the
 * plaintext). The frontend substitutes the key it holds from the provision response. Must match
 * `API_KEY_PLACEHOLDER` in backend/src/server/daemons/impl/install_artifacts.rs.
 */
export const API_KEY_PLACEHOLDER = '<API_KEY>';

/**
 * The install command the create wizard shows for an OS and Linux install method, or null until
 * it can show the real one. Only the server's command is ever shown: before the install-command
 * response and the minted key both exist there is nothing to display, never a stand-in built
 * from form values.
 */
export function installStepCommand(
	artifacts: InstallArtifacts | null | undefined,
	apiKey: string | null,
	os: OsInstallMethod,
	linuxMethod: 'binary' | 'docker'
): string | null {
	if (!artifacts || !apiKey) return null;
	const filled = fillInstallArtifactsKey(artifacts, apiKey);
	if (os === 'linux' && linuxMethod === 'docker') return filled.docker.compose ?? null;
	return osInstallCommand(filled, os) || null;
}

/** Fill the api-key placeholder in every install method that carries a command. */
export function fillInstallArtifactsKey(
	artifacts: InstallArtifacts,
	apiKey: string
): InstallArtifacts {
	const sub = (s: string) => s.replaceAll(API_KEY_PLACEHOLDER, apiKey);
	return {
		...artifacts,
		linux: sub(artifacts.linux),
		macos: sub(artifacts.macos),
		windows: sub(artifacts.windows),
		freebsd: sub(artifacts.freebsd),
		docker: {
			...artifacts.docker,
			compose: artifacts.docker.compose ? sub(artifacts.docker.compose) : artifacts.docker.compose
		}
	};
}
