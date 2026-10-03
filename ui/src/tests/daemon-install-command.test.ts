import { describe, expect, it } from 'vitest';
import {
	API_KEY_PLACEHOLDER,
	installStepCommand,
	type InstallArtifacts,
	type OsInstallMethod
} from '$lib/features/daemons/types/base';

const KEY = 'scp_d_minted';

/** Server artifacts as the install-command endpoint returns them: each method its own string. */
const artifacts: InstallArtifacts = {
	linux: `linux-cmd --daemon-api-key ${API_KEY_PLACEHOLDER}`,
	macos: `macos-cmd --daemon-api-key ${API_KEY_PLACEHOLDER}`,
	windows: `windows-cmd --daemon-api-key ${API_KEY_PLACEHOLDER}`,
	freebsd: `freebsd-cmd --daemon-api-key ${API_KEY_PLACEHOLDER}`,
	docker: { compose: `SCANOPY_DAEMON_API_KEY=${API_KEY_PLACEHOLDER}`, env: [] },
	msi: { filename: 'scanopy-daemon-x.msi', omitted_config_keys: [] }
};

const methods: [OsInstallMethod, 'binary' | 'docker', string][] = [
	['linux', 'binary', artifacts.linux],
	['linux', 'docker', artifacts.docker.compose ?? ''],
	['macos', 'binary', artifacts.macos],
	['windows', 'binary', artifacts.windows],
	['freebsd', 'binary', artifacts.freebsd]
];

describe('installStepCommand', () => {
	it('shows nothing before the server command and the minted key both exist', () => {
		for (const [os, linuxMethod] of methods) {
			// Provisioned, install-command fetch still in flight: the state that used to flash a
			// command built from form values.
			expect(installStepCommand(null, KEY, os, linuxMethod)).toBeNull();
			// Command fetched but no key: the state a reopened wizard used to get stuck in.
			expect(installStepCommand(artifacts, null, os, linuxMethod)).toBeNull();
		}
	});

	it("shows the server's command for every OS and install method, key filled", () => {
		for (const [os, linuxMethod, serverCommand] of methods) {
			const shown = installStepCommand(artifacts, KEY, os, linuxMethod);
			expect(shown).toBe(serverCommand.replaceAll(API_KEY_PLACEHOLDER, KEY));
			expect(shown).not.toContain(API_KEY_PLACEHOLDER);
		}
	});
});
