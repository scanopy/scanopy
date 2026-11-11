import { describe, it, expect } from 'vitest';
import warningCodes from '#lib/data/warning-codes.json';
import warningRemedies from '#lib/data/warning-remedies.json';
import discoveryIntegrations from '#lib/data/discovery-integrations.json';
import claimSources from '#lib/data/claim-sources.json';
import type { components } from '#lib/api/schema';
import { snmpCheckOf, type CredentialLookup } from '#lib/features/discovery/utils/snmpCheck.ts';
import {
	buildWarningReport,
	credentialIdsOf,
	type DiscoveryWarning,
	type EntityNameLookup
} from '#lib/features/discovery/utils/warnings.ts';

type Credential = components['schemas']['Credential'];

/**
 * The frontend half of the slot contract.
 *
 * A code's sentence is a template with `{named}` slots, and three things have to agree on those
 * names: the Rust `description()`, the `slots()` the fixture publishes, and the parameters this
 * renderer passes to the paraglide message. The backend test
 * (`every_description_interpolates_exactly_the_slots_it_declares`) pins the first two together;
 * this pins the third to them. Without it, a renderer that forgets a slot compiles fine and shows
 * the operator a literal `{addresses}`.
 */
describe('discovery warning rendering', () => {
	/** One warning per code, with every field its variant carries. */
	const sample = (code: string): DiscoveryWarning =>
		({
			code,
			address: '10.0.0.1',
			collected: 3,
			group: 'Lldp',
			limit: 10000,
			source: 'IfNumber',
			expected: 23,
			observed: 1,
			dropped: 1,
			total: 4,
			misplaced: 2,
			discarded: 14,
			kept: 0,
			consequence: 'AllLinksLost',
			integration: 'Snmp',
			ports: [443],
			detail: 'diagnostic',
			hours: 4,
			hosts_not_scanned: 12,
			minutes_remaining: 40,
			credential_id: '00000000-0000-0000-0000-0000000000c1',
			host_id: '00000000-0000-0000-0000-000000000001',
			remote_host_id: '00000000-0000-0000-0000-000000000002',
			if_descr: 'Gi1/0/1',
			identifier: '00:ad:24:89:cc:f0',
			sys_name: 'core-sw',
			port_id: 'MacAddress("00:ad:24:af:4e:00")',
			port_desc: 'Port 9',
			cidr: '10.20.30.0/24',
			declined: 254,
			subnet_id: '00000000-0000-0000-0000-000000000003',
			addresses: ['10.20.30.11'],
			sys_names: ['offsite-core-01'],
			seen_by_host_ids: ['00000000-0000-0000-0000-000000000001'],
			widened_by_vlan: false,
			declared: 'Windows',
			actual: 'Unix',
			elided: 7,
			cause: 'TimedOut',
			failures: [{ group: 'Interfaces', cause: 'TimedOut' }],
			gap: {
				type: 'TargetsContainerBridgeWithCredentials',
				details: { addresses: ['172.17.0.3'] }
			},
			last_unsupported_daemon_version: '0.18.2',
			guest: 'gitlab',
			node: 'pve',
			bridge: 'vmbr1',
			vm: 'DockerDesktop'
		}) as unknown as DiscoveryWarning;

	/**
	 * Every sentence a run renders, in reading order.
	 *
	 * The report is grouped now — sections of rows, each row holding its code's sentences — but
	 * the slot contract these tests pin is about the sentences themselves, so they read them flat.
	 */
	const sentences = (warnings: DiscoveryWarning[], names?: EntityNameLookup): string[] =>
		buildWarningReport(warnings, names)
			.flatMap((section) => section.entries)
			.flatMap((entry) => entry.details)
			.map((statement) => statement.sentence);

	/** Every example line a run renders, flattened. */
	const examplesOf = (warnings: DiscoveryWarning[], names?: EntityNameLookup) =>
		buildWarningReport(warnings, names)
			.flatMap((section) => section.entries)
			.flatMap((entry) => entry.details)
			.flatMap((statement) => statement.examples);

	/**
	 * A credential withheld whole and one that lost only its bridge addresses are different
	 * statements, so each gap renders as its own sentence, and none leaves a slot unfilled.
	 */
	it('renders each capability gap as its own sentence', () => {
		const gaps = [
			{ type: 'ParsesCredentialType' },
			{
				type: 'UsesSnmpV3SecurityOptions',
				details: { options: ['NoAuthNoPriv', 'AuthNoPriv', 'Md5Auth', 'DesPrivacy'] }
			},
			{ type: 'TargetsContainerBridgeWithCredentials', details: { addresses: ['172.17.0.3'] } }
		];
		const rendered = sentences(
			gaps.map(
				(gap) => ({ ...sample('CredentialNeedsNewerDaemon'), gap }) as unknown as DiscoveryWarning
			)
		);

		expect(rendered).toHaveLength(gaps.length);
		for (const sentence of rendered) expect(sentence).not.toMatch(/\{\w+\}/);
	});

	/**
	 * A run recorded before daemons sent their OS carries only the family. The daemon's own
	 * reported OS names it then, and the family is the last resort.
	 */
	it("names the daemon's own OS where the warning carries only its family", () => {
		const mismatch = {
			...sample('CredentialDaemonOsMismatch'),
			actual_os: null
		} as DiscoveryWarning;
		const named = (os: 'linux' | null) =>
			buildWarningReport([mismatch], undefined, { name: 'HQ Daemon', os })
				.flatMap((section) => section.entries)
				.flatMap((entry) => entry.details)
				.map((statement) => statement.sentence)[0];

		expect(named('linux')).toContain('HQ Daemon runs Linux,');
		expect(named(null)).toContain('HQ Daemon runs Linux, macOS or BSD,');
	});

	it('renders every code the backend can send, with no slot left unfilled', () => {
		const unfilled: string[] = [];

		for (const entry of warningCodes) {
			const rendered = sentences([sample(entry.id)]);

			expect(rendered, `${entry.id} rendered nothing`).toHaveLength(1);
			// A `{slot}` surviving into the output is a parameter the renderer did not supply.
			const holes = rendered[0].match(/\{\w+\}/g);
			if (holes) {
				unfilled.push(`  ${entry.id}: ${holes.join(', ')}`);
			}
		}

		if (unfilled.length > 0) {
			expect.fail(
				`Warning templates rendered with unfilled slots:\n\n${unfilled.join('\n')}\n\n` +
					'Add the missing parameters to WARNING_PARAMS in ' +
					'src/lib/features/discovery/utils/warnings.ts.'
			);
		}
	});

	it('names the desktop VM a daemon runs in by its product name', () => {
		const [sentence] = sentences([sample('DaemonInDesktopVm')]);
		expect(sentence).toContain('Docker Desktop');
		expect(sentence).not.toContain('{vm}');
	});

	it('groups warnings sharing a code into one sentence, naming every address', () => {
		// A code with no device check, so the sentence is the only place its devices are named.
		const warnings = ['192.168.7.235', '192.168.7.242'].map((address) => ({
			...sample('VlanRecordingFailed'),
			address
		}));

		const rendered = sentences(warnings as DiscoveryWarning[]);

		// The reported problem this aggregation exists for: fifteen switches produced fifteen
		// paragraphs. One sentence per code, always — and no device silently dropped from it.
		expect(rendered).toHaveLength(1);
		expect(rendered[0]).toContain('192.168.7.235');
		expect(rendered[0]).toContain('192.168.7.242');
	});

	it('shares a sentence between devices whose reads all ended the same way, and only then', () => {
		const nothing = (address: string, failures: unknown) =>
			({ ...sample('SnmpCollectedNothing'), address, failures }) as DiscoveryWarning;
		const timedOut = [
			{ group: 'Interfaces', cause: 'TimedOut' },
			{ group: 'ArpTable', cause: 'TimedOut' }
		];

		const same = sentences([nothing('10.91.16.1', timedOut), nothing('10.91.16.11', timedOut)]);
		const different = sentences([
			nothing('10.91.16.1', timedOut),
			nothing('10.91.16.11', [{ group: 'Interfaces', cause: 'GenErr' }])
		]);

		// The customer's two devices failed identically, and read as one line naming both.
		expect(same).toHaveLength(1);
		expect(same[0]).toContain('10.91.16.1');
		expect(same[0]).toContain('10.91.16.11');
		// A device whose reads ended differently is a different statement.
		expect(different).toHaveLength(2);
	});

	it('keeps devices that failed differently in separate sentences', () => {
		const rendered = sentences([
			sample('SnmpWalkNoAnswer'),
			{ ...sample('SnmpWalkUnsupported'), address: '10.0.0.2' } as DiscoveryWarning
		]);

		// One says a rescan may help and the other says it never will; merging them would have to
		// pick one answer and be wrong for the other device.
		expect(rendered).toHaveLength(2);
	});

	it('resolves every integration to a name, never a raw discriminant', () => {
		// The discriminants a warning can carry are not the ids of credential-types.json, which is
		// keyed by CredentialType — reusing that fixture resolved DockerProxy by coincidence and
		// rendered Snmp, UnifiController and InstantOn as their raw values.
		const integrations = discoveryIntegrations.map((i) => i.id);
		expect(integrations.length).toBeGreaterThan(0);

		for (const id of integrations) {
			const w = { ...sample('CredentialRejected'), integration: id } as unknown as DiscoveryWarning;
			const [line] = sentences([w]);
			const name = discoveryIntegrations.find((i) => i.id === id)?.name;
			expect(line, `${id} has no display name`).toContain(name);
			// The raw value may only appear as part of the display name ("Proxmox" in "Proxmox VE API").
			expect(line.replace(name ?? '', ''), `${id} leaked its discriminant`).not.toContain(id);
		}
	});

	it('gives each failing credential address its own sentence and diagnostic', () => {
		const at = (address: string, detail: string) =>
			({ ...sample('CredentialRejected'), address, detail }) as unknown as DiscoveryWarning;

		const rendered = sentences([
			at('10.0.0.1', 'wrong community'),
			at('10.0.0.2', 'authentication failure')
		]);

		// Grouping these would put one diagnostic against both addresses, which is the batching the
		// per-occurrence records exist to undo.
		expect(rendered).toHaveLength(2);
		expect(rendered[0]).toContain('10.0.0.1');
		expect(rendered[0]).toContain('wrong community');
		expect(rendered[1]).toContain('10.0.0.2');
		expect(rendered[1]).toContain('authentication failure');
	});

	it('survives a scan whose hosts have since been deleted', () => {
		// Host names are resolved live, but a historical record outlives the hosts it names. With
		// no name to show, omitting the segment can leave the arrow with nothing on one side —
		// "-> TAMMIERENEW", or "1/1 -> via InterfaceName(…)" — which reads as a rendering fault
		// rather than as missing data.
		const noNames = () => undefined;

		for (const code of [
			'LldpNeighbourNotFound',
			'LldpNeighbourAmbiguous',
			'LldpPortNoStrategy',
			'LldpPortNotFound',
			'LldpPortAmbiguous'
		]) {
			// The worst case: no host name *and* a device that reported no interface description.
			const w = { ...sample(code), if_descr: '' } as unknown as DiscoveryWarning;
			const [line] = sentences([w], noNames);

			expect(line, `${code} left a dangling arrow`).not.toMatch(/(^|[,:.] |\band )+->/);
			expect(line, `${code} left a trailing arrow`).not.toMatch(/->\s*$/);
			expect(line, `${code} leaked a host id`).not.toMatch(
				/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/
			);
			expect(line).not.toMatch(/\{\w+\}/);
		}
	});

	it('names a host deleted since the scan instead of dropping its chip', () => {
		// A historical record outlives the hosts it names, and deleting one used to make its chip
		// vanish — eight unresolved pairs rendered as eight bare port descriptions with nothing to
		// say which devices they concerned. `null` is the lookup saying it ran and the host is gone.
		const gone = () => null;

		for (const code of ['LldpNeighbourNotFound', 'LldpPortNotFound']) {
			const [line] = examplesOf([sample(code)], gone);

			expect(line.near?.label, `${code} dropped its near device`).toMatch(/unknown/i);
			// No entity: there is nothing left to navigate to, so it reads as a label.
			expect(line.near?.entity).toBeUndefined();
			expect(JSON.stringify(line), `${code} leaked a host id`).not.toMatch(
				/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/
			);
		}

		// Both ends of a resolved-port pair, so a line never half-names itself.
		const [pair] = examplesOf([sample('LldpPortNotFound')], gone);
		expect(pair.far?.label).toMatch(/unknown/i);
	});

	it('still omits a chip while the host lookup is in flight', () => {
		// The distinction the tri-state exists for: `undefined` is "not known yet", and must not
		// flash "unknown" on the way to a name.
		const notYet = () => undefined;
		const [line] = examplesOf([sample('LldpPortNotFound')], notYet);

		expect(line.near).toBeNull();
		expect(line.far).toBeNull();
	});

	it('renders a legacy string warning as its own text', () => {
		const legacy = {
			code: 'Unknown',
			detail: 'Scan hit its time limit (4h) — 12 host(s) not scanned.'
		} as unknown as DiscoveryWarning;

		// Historical sessions hold bare sentences, and they have to keep reading exactly as they
		// did before warnings were coded.
		expect(sentences([legacy])).toEqual(['Scan hit its time limit (4h) — 12 host(s) not scanned.']);
	});

	it('files every code under a rung the remedy registry defines', () => {
		// Two fixtures have to agree for the report to render at all: a code names its rung in
		// `category`, and the rung supplies the heading. A code filed under a rung that is not
		// there renders in no section and vanishes from the list without an error.
		const rungs = new Set(warningRemedies.map((r) => r.id));
		expect(rungs.size).toBeGreaterThan(0);

		const orphaned = warningCodes
			.filter((code) => !code.category || !rungs.has(code.category))
			.map((code) => `  ${code.id}: ${code.category ?? 'no category'}`);

		if (orphaned.length > 0) {
			expect.fail(
				`Warning codes filed under a rung warning-remedies.json does not define:\n\n` +
					`${orphaned.join('\n')}\n\nRe-run \`make generate-fixtures\`.`
			);
		}
	});

	it('collapses one problem on many devices into a single sentence naming each of them', () => {
		// The wall of text this replaced: four switches restricting the same SNMP view produced
		// four near-identical paragraphs differing only in the address. The claim carries no
		// per-device figure, so there is nothing to keep them apart.
		const devices = ['192.168.7.248', '192.168.7.252', '192.168.7.253', '192.168.7.254'];
		const claim = (address: string, source: string, group: string) =>
			({ code: 'ClaimedCapabilityEmpty', address, source, group }) as unknown as DiscoveryWarning;

		const report = buildWarningReport(
			devices.map((address) => claim(address, 'SysServicesBridgeBit', 'BridgePortNumbering'))
		);

		expect(report).toHaveLength(1);
		expect(report[0].entries).toHaveLength(1);
		expect(report[0].entries[0].affects).toEqual({ type: 'address', count: 4, only: null });
		expect(report[0].entries[0].details).toHaveLength(1);
		for (const device of devices) {
			expect(report[0].entries[0].details[0].sentence).toContain(device);
		}
	});

	it('keeps a device apart when its statement differs, not just its address', () => {
		// The other half of the same rule. A device that advertised a different capability, or
		// reported different figures, is making a different statement — merging those would let
		// one set of numbers speak for a device that never reported them.
		const claim = (address: string, source: string, group: string) =>
			({ code: 'ClaimedCapabilityEmpty', address, source, group }) as unknown as DiscoveryWarning;

		const [section] = buildWarningReport([
			claim('192.168.7.248', 'SysServicesBridgeBit', 'BridgePortNumbering'),
			claim('192.168.7.252', 'SysServicesBridgeBit', 'BridgePortNumbering'),
			claim('192.168.7.236', 'LldpLocalIdentity', 'Lldp')
		]);

		// Still one row — it is one code — but two sentences under it.
		expect(section.entries).toHaveLength(1);
		expect(section.entries[0].details).toHaveLength(2);
		expect(section.entries[0].details[0].sentence).toContain('192.168.7.248');
		expect(section.entries[0].details[0].sentence).toContain('192.168.7.252');
		expect(section.entries[0].details[1].sentence).toContain('192.168.7.236');
		expect(section.entries[0].details[1].sentence).not.toContain('192.168.7.248');
	});

	/**
	 * The gap this closes: a *Fix in Scanopy* row asks the reader to change something, and until
	 * the warning carried an id the row could only offer the credential list to go and find it in.
	 * On a network with two SNMP communities the address does not narrow it to one.
	 */
	describe('naming the credential a row is about', () => {
		const CREDENTIAL = '00000000-0000-0000-0000-0000000000c1';
		const OTHER_CREDENTIAL = '00000000-0000-0000-0000-0000000000c2';

		/** Names only the credentials, so an unresolved one is distinguishable from a missing id. */
		const credentialNames: EntityNameLookup = (type, id) =>
			type === 'Credential' && id === CREDENTIAL ? 'Core switches SNMP' : undefined;

		/**
		 * A credential warning with exactly the fields its variant carries.
		 *
		 * Not the maximal `sample()`: that object holds every field of every variant, including a
		 * `host_id`, and a chip is read off the payload — so a credential row built from it would
		 * take the host branch and prove nothing about credentials.
		 */
		const credentialWarning = (
			code: string,
			extra: Record<string, unknown> = {}
		): DiscoveryWarning =>
			({
				code,
				address: '10.0.0.1',
				integration: 'Snmp',
				detail: 'diagnostic',
				credential_id: CREDENTIAL,
				...extra
			}) as unknown as DiscoveryWarning;

		const rowFor = (warnings: DiscoveryWarning[]) =>
			buildWarningReport(warnings, credentialNames).flatMap((s) => s.entries)[0];

		it('counts the address a credential failed at, and keeps the credential as the action target', () => {
			const row = rowFor([credentialWarning('CredentialMalformed')]);

			expect(row.affects).toEqual({ type: 'address', count: 1, only: '10.0.0.1' });
			expect(credentialIdsOf(row)).toEqual([CREDENTIAL]);
		});

		it('renders a warning recorded before ids existed with the address alone', () => {
			// A historical row. The field is absent rather than null, which is what an older
			// daemon posts too — neither may lose the row or leak a placeholder chip.
			const withoutId = { ...(credentialWarning('CredentialMalformed') as object) } as Record<
				string,
				unknown
			>;
			delete withoutId.credential_id;
			const row = rowFor([withoutId as unknown as DiscoveryWarning]);

			expect(row.affects?.only).toBe('10.0.0.1');
			expect(row.details[0].sentence).not.toMatch(/\{\w+\}/);
			expect(credentialIdsOf(row)).toEqual([]);
		});

		it('keeps a credential whose name did not resolve as the action target', () => {
			// Deleted since the scan, or a viewer who cannot read credentials.
			const row = rowFor([
				credentialWarning('CredentialMalformed', { credential_id: OTHER_CREDENTIAL })
			]);

			// The id is good even if the name is not loaded, and the editor resolves it itself.
			expect(credentialIdsOf(row)).toEqual([OTHER_CREDENTIAL]);
		});

		it('names every credential a row implicates, deduped and in first-seen order', () => {
			// Order carries weight: the row's action opens the first of these, so a row covering
			// several credentials still lands the reader on one of the records it is about rather
			// than on the list. The rest stay reachable through the row's chips.
			const at = (address: string, credential_id: string) =>
				credentialWarning('CredentialRejected', { address, credential_id });

			expect(credentialIdsOf(rowFor([at('10.0.0.1', CREDENTIAL)]))).toEqual([CREDENTIAL]);

			const several = rowFor([at('10.0.0.2', OTHER_CREDENTIAL), at('10.0.0.1', CREDENTIAL)]);
			expect(credentialIdsOf(several)).toEqual([OTHER_CREDENTIAL, CREDENTIAL]);

			// Deduped: one broken credential met at twenty addresses is still one credential.
			const repeated = rowFor([at('10.0.0.1', CREDENTIAL), at('10.0.0.2', CREDENTIAL)]);
			expect(credentialIdsOf(repeated)).toEqual([CREDENTIAL]);
		});
	});

	it('merges credential failures that returned the same diagnostic', () => {
		// The complement of "each failing credential gets its own diagnostic": when the diagnostic
		// is identical there is nothing per-address left to lose, and repeating the sentence once
		// per host is the batching this grouping exists to undo.
		const at = (address: string, detail: string) =>
			({ ...sample('CredentialRejected'), address, detail }) as unknown as DiscoveryWarning;

		const rendered = sentences([
			at('10.0.0.1', 'wrong community'),
			at('10.0.0.2', 'wrong community'),
			at('10.0.0.3', 'authentication failure')
		]);

		expect(rendered).toHaveLength(2);
		expect(rendered[0]).toContain('10.0.0.1');
		expect(rendered[0]).toContain('10.0.0.2');
		expect(rendered[1]).toContain('10.0.0.3');
	});

	it('gives each integration its own row, naming all its hosts in one sentence', () => {
		const found = (integration: string, n: number) =>
			Array.from(
				{ length: n },
				(_, i) =>
					({
						code: 'IntegrationWithoutCredential',
						integration,
						host_id: `00000000-0000-0000-0000-${String(i).padStart(12, '0')}`
					}) as unknown as DiscoveryWarning
			);

		const rows = buildWarningReport([...found('Snmp', 40), ...found('Proxmox', 1)]).flatMap(
			(section) => section.entries
		);

		// A row per integration, so each has its own title and its own button, and forty SNMP
		// devices are still one row and one sentence rather than forty.
		expect(rows).toHaveLength(2);
		const [snmp, proxmox] = rows;
		expect(snmp.title).toContain('SNMP');
		expect(proxmox.title).toContain('Proxmox');
		expect(snmp.warnings).toHaveLength(40);
		expect(snmp.details).toHaveLength(1);
		expect(snmp.details[0].sentence).toContain('40');
		expect(proxmox.details[0].sentence).not.toContain('40');
		expect(new Set(rows.map((r) => r.key)).size).toBe(2);
	});

	// Hosts are told apart by id, not name: two hosts both named "Docker" are two hosts.
	it('counts same-named hosts separately', () => {
		const onHost = (host_id: string) =>
			({
				code: 'IntegrationWithoutCredential',
				integration: 'Docker',
				host_id
			}) as unknown as DiscoveryWarning;
		const sameName: EntityNameLookup = () => 'Docker';

		const [row] = buildWarningReport(
			[
				onHost('00000000-0000-0000-0000-000000000001'),
				onHost('00000000-0000-0000-0000-000000000002')
			],
			sameName
		).flatMap((section) => section.entries);

		expect(row.affects).toEqual({ type: 'host', count: 2, only: null });
	});
});

/**
 * The commands a row offers for checking a warning against its device.
 *
 * Each device gets its own, built from its own address and the stored credential its warning
 * names; a device with none says why. Asserted on what the commands do (which device, which
 * request, which credential), not on their exact text.
 */
describe('SNMP check commands', () => {
	const credentialId = '00000000-0000-0000-0000-0000000000c1';
	const v3Credential = {
		id: credentialId,
		credential_type: {
			type: 'SnmpV3',
			security_name: 'netops',
			security_level: 'AuthPriv',
			auth_protocol: 'Sha256',
			auth_password: { mode: 'Inline', value: 'auth-secret' },
			priv_protocol: 'Aes256',
			priv_password: { mode: 'Inline', value: 'priv-secret' },
			context_name: 'vlan-20'
		}
	} as unknown as Credential;
	const credentials: CredentialLookup = (id) => (id === credentialId ? v3Credential : undefined);

	const nothing = (address: string, failures: unknown) =>
		({
			code: 'SnmpCollectedNothing',
			address,
			failures,
			credential_id: credentialId
		}) as unknown as DiscoveryWarning;

	const outcomesOf = (warnings: DiscoveryWarning[]) =>
		buildWarningReport(warnings)
			.flatMap((section) => section.entries)
			.flatMap((entry) => entry.details)
			.map((statement) => statement.warnings.map((w) => snmpCheckOf(w, credentials)));

	it('gives each device its own commands, and a device with none the reason why', () => {
		const statements = outcomesOf([
			nothing('192.168.7.229', [{ group: 'Interfaces', cause: 'TimedOut' }]),
			nothing('192.168.7.230', [{ group: 'Interfaces', cause: 'GenErr' }]),
			nothing('192.168.7.212', [])
		]);

		expect(statements).toHaveLength(3);
		const [silent, generr, empty] = statements.map(([outcome]) => outcome);
		for (const [outcome, address] of [
			[silent, '192.168.7.229'],
			[generr, '192.168.7.230']
		] as const) {
			expect(outcome && 'commands' in outcome).toBe(true);
			const commands = (outcome as { commands: string[] }).commands;
			expect(commands.every((c) => c.includes(address))).toBe(true);
		}
		expect(empty && 'reason' in empty).toBe(true);
	});

	it('authenticates with the stored credential, and never prints its secrets', () => {
		const [[outcome]] = outcomesOf([nothing('10.0.0.1', [{ group: 'Lldp', cause: 'TimedOut' }])]);
		const commands = (outcome as { commands: string[] }).commands.join('\n');

		expect(commands).toContain('-u netops');
		expect(commands).toContain('-n vlan-20');
		expect(commands).not.toContain('secret');
	});

	it('turns off the increasing-OID check for an agent that answered out of order', () => {
		const [[outcome]] = outcomesOf([
			{
				code: 'SnmpWalkDesynchronised',
				address: '10.0.0.1',
				group: 'ArpTable',
				credential_id: credentialId
			} as unknown as DiscoveryWarning
		]);

		expect((outcome as { commands: string[] }).commands.every((c) => c.includes('-Cc'))).toBe(true);
	});

	it("reads a device's own claim before recounting what it served", () => {
		const [[outcome]] = outcomesOf([
			{
				code: 'ClaimedCountUnderRead',
				address: '10.0.0.1',
				group: 'Interfaces',
				source: 'IfNumber',
				expected: 23,
				observed: 1,
				credential_id: credentialId
			} as unknown as DiscoveryWarning
		]);
		const claimed = claimSources.find((c) => c.id === 'IfNumber')?.metadata as { oid: string };

		const [first] = (outcome as { commands: string[] }).commands;
		expect(first.startsWith('snmpget')).toBe(true);
		expect(first).toContain(claimed.oid);
	});

	it('offers no command for a device read without a known credential', () => {
		const [[outcome]] = outcomesOf([
			{
				code: 'SnmpWalkPartialRecorded',
				address: '10.0.0.9',
				group: 'Lldp',
				credential_id: null
			} as unknown as DiscoveryWarning
		]);

		expect(outcome && 'commands' in outcome).toBe(false);
		expect(outcome && 'reason' in outcome && outcome.reason).toContain('10.0.0.9');
	});

	it('takes a row with commands one device at a time, each with its own sentence', () => {
		const claim = (address: string, credential_id: string | null) =>
			({
				code: 'ClaimedCapabilityEmpty',
				address,
				source: 'SysServicesBridgeBit',
				group: 'BridgePortNumbering',
				credential_id
			}) as unknown as DiscoveryWarning;

		const [row] = buildWarningReport(
			[claim('192.168.7.212', credentialId), claim('192.168.7.218', null)],
			undefined,
			null,
			credentials
		).flatMap((s) => s.entries);

		expect(row.devices.map((d) => d.address)).toEqual(['192.168.7.212', '192.168.7.218']);
		const [withCommands, without] = row.devices;
		expect(withCommands.sentence).toContain('192.168.7.212');
		expect(withCommands.sentence).not.toContain('192.168.7.218');
		expect('commands' in withCommands.outcome).toBe(true);
		expect('reason' in without.outcome).toBe(true);
	});

	it('lists every device when no device in the row has commands', () => {
		const claim = (address: string) =>
			({
				code: 'ClaimedCapabilityEmpty',
				address,
				source: 'SysServicesBridgeBit',
				group: 'BridgePortNumbering',
				credential_id: null
			}) as unknown as DiscoveryWarning;

		const [row] = buildWarningReport(
			[claim('192.168.7.212'), claim('192.168.7.218')],
			undefined,
			null,
			credentials
		).flatMap((s) => s.entries);

		expect(row.devices).toEqual([]);
		expect(row.details[0].sentence).toContain('192.168.7.212');
		expect(row.details[0].sentence).toContain('192.168.7.218');
	});

	it('offers nothing for a code no device read can repeat', () => {
		const [[outcome]] = outcomesOf([
			{
				code: 'VlanRecordingFailed',
				address: '10.0.0.1'
			} as unknown as DiscoveryWarning
		]);

		expect(outcome).toBeNull();
	});

	it('checks a community credential over the version the device answered on', () => {
		const communityId = '00000000-0000-0000-0000-0000000000c2';
		const community = {
			id: communityId,
			credential_type: { type: 'SnmpCommunity', community: { mode: 'Inline', value: 'x' } }
		} as unknown as Credential;
		const commandsFor = (snmp_version: string | null) => {
			const outcome = snmpCheckOf(
				{
					code: 'SnmpWalkFailed',
					address: '10.0.0.5',
					group: 'Interfaces',
					cause: 'TimedOut',
					credential_id: communityId,
					snmp_version
				} as unknown as DiscoveryWarning,
				(id) => (id === communityId ? community : undefined)
			);
			return (outcome as { commands: string[] }).commands;
		};

		// SNMPv1 has no GETBULK, so a v1 device gets only walks that can reach it.
		const v1 = commandsFor('V1');
		expect(v1.length).toBeGreaterThan(0);
		expect(v1.every((c) => c.startsWith('snmpwalk ') && c.includes('-v1 '))).toBe(true);

		for (const version of ['V2c', null]) {
			expect(commandsFor(version).every((c) => c.includes('-v2c '))).toBe(true);
		}
	});
});

/** What a collapsed row says it concerns, in place of a strip of chips. */
describe('warning row header counts', () => {
	const rowOf = (warnings: object[]) =>
		buildWarningReport(warnings as DiscoveryWarning[]).flatMap((s) => s.entries)[0];

	it('counts the hosts a neighbour row is about, by the device that reported it', () => {
		const port = (host_id: string) => ({
			code: 'LldpPortNotFound',
			host_id,
			remote_host_id: null,
			if_descr: 'Gi1/0/1',
			port_id: 'Port 9',
			count: 1
		});

		const row = rowOf([
			port('00000000-0000-0000-0000-000000000001'),
			port('00000000-0000-0000-0000-000000000002')
		]);

		expect(row.affects).toMatchObject({ type: 'host', count: 2 });
	});

	it('gives a scan-wide row no count', () => {
		const row = rowOf([{ code: 'ScanTimeLimit', hours: 4, hosts_not_scanned: 12 }]);

		expect(row.affects).toBeNull();
	});
});

/** The lines under a row that name each thing it is about, as links where there is one. */
describe('warning row example links', () => {
	it('links an assumed range to its subnet', () => {
		const [line] = buildWarningReport([
			{
				code: 'ProvisionalSubnetInferred',
				cidr: '10.20.31.0/24',
				subnet_id: '00000000-0000-0000-0000-000000000003',
				addresses: ['10.20.31.9'],
				sys_names: ['offsite-branch-01'],
				seen_by_host_ids: [],
				widened_by_vlan: false,
				count: 1
			} as unknown as DiscoveryWarning
		])
			.flatMap((s) => s.entries)
			.flatMap((e) => e.details)
			.flatMap((d) => d.examples);

		expect(line.near).toEqual({
			label: '10.20.31.0/24',
			entity: { type: 'Subnet', id: '00000000-0000-0000-0000-000000000003' }
		});
		expect(line.nearText).not.toContain('10.20.31.0/24');
	});

	it('lists the hosts an integration was found on without a credential', () => {
		const hostNames: EntityNameLookup = (type, id) =>
			type === 'Host' ? `host-${id.slice(-1)}` : undefined;
		const onHost = (host_id: string) =>
			({
				code: 'IntegrationWithoutCredential',
				integration: 'Snmp',
				host_id
			}) as unknown as DiscoveryWarning;

		const lines = buildWarningReport(
			[
				onHost('00000000-0000-0000-0000-000000000001'),
				onHost('00000000-0000-0000-0000-000000000002')
			],
			hostNames
		)
			.flatMap((s) => s.entries)
			.flatMap((e) => e.details)
			.flatMap((d) => d.examples);

		expect(lines.map((l) => l.near?.label)).toEqual(['host-1', 'host-2']);
		expect(lines.every((l) => l.near?.entity?.type === 'Host')).toBe(true);
	});
});
