import { describe, expect, it } from 'vitest';
import { discoveredName, nameToSubmit, overrideOf } from '$lib/features/hosts/host-identity';
import type { HostNameLadderEntry } from '$lib/features/hosts/types/base';

/**
 * The host editor's split of a stored name into a discovered name and a person's override.
 *
 * The ladders below stand in for the server's `name_ladder`: every rung in the server's order,
 * blanks already dropped, sources only where there is a value.
 */

const SNMP = { Probe: 'Snmp' } as const;

function ladder(
	values: Partial<Record<HostNameLadderEntry['rung'], [string, HostNameLadderEntry['source']]>>
): HostNameLadderEntry[] {
	return (['Name', 'Hostname', 'SysName', 'ChassisId', 'Address'] as const).map((rung) => ({
		rung,
		value: values[rung]?.[0] ?? null,
		source: values[rung]?.[1] ?? null
	}));
}

describe('discoveredName', () => {
	it('is the stored name when discovery wrote it', () => {
		expect(
			discoveredName(ladder({ Name: ['core-sw-01', SNMP], Hostname: ['switch.lan', null] }))
		).toEqual({ value: 'core-sw-01', rung: 'Name', source: SNMP });
	});

	it('looks past a name a person set, to what discovery would call the host', () => {
		expect(
			discoveredName(ladder({ Name: ['Core Switch', 'Manual'], Hostname: ['switch.lan', null] }))
		).toEqual({ value: 'switch.lan', rung: 'Hostname', source: null });
	});

	it('treats an unattributed stored name as discovered, not as an override', () => {
		expect(discoveredName(ladder({ Name: ['nas.lan', 'Unspecified'] }))).toEqual({
			value: 'nas.lan',
			rung: 'Name',
			source: 'Unspecified'
		});
	});

	it('falls to the sysName of a nameless host', () => {
		expect(
			discoveredName(ladder({ SysName: ['printer-hp-main', SNMP], Address: ['10.0.30.50', null] }))
		).toEqual({ value: 'printer-hp-main', rung: 'SysName', source: SNMP });
	});

	it('falls to the address of a host with nothing else', () => {
		expect(discoveredName(ladder({ Address: ['10.0.30.61', null] }))).toEqual({
			value: '10.0.30.61',
			rung: 'Address',
			source: null
		});
	});

	it('follows the server order, which puts a guessed name below the identifiers', () => {
		const serverOrder: HostNameLadderEntry[] = [
			{ rung: 'Hostname', value: 'nas.lan', source: 'ReverseDns' },
			{ rung: 'SysName', value: null, source: null },
			{ rung: 'ChassisId', value: null, source: null },
			{ rung: 'Name', value: 'SSH', source: 'ServiceMatch' },
			{ rung: 'Address', value: '10.0.0.5', source: null }
		];
		expect(discoveredName(serverOrder)).toEqual({
			value: 'nas.lan',
			rung: 'Hostname',
			source: 'ReverseDns'
		});
	});

	it('is null when nothing identifies the host', () => {
		expect(discoveredName(ladder({}))).toBeNull();
		expect(discoveredName(ladder({ Name: ['Rack 3', 'Manual'] }))).toBeNull();
	});
});

describe('the override round trip', () => {
	it('shows only a name a person set as the override', () => {
		expect(overrideOf('Core Switch', 'Manual')).toBe('Core Switch');
		expect(overrideOf('core-sw-01', SNMP)).toBe('');
	});

	it('leaves a discovered name untouched when the override stays blank', () => {
		const saved = { savedName: 'core-sw-01', nameSource: SNMP };
		expect(
			nameToSubmit({ override: overrideOf(saved.savedName, saved.nameSource), ...saved })
		).toBe('core-sw-01');
	});

	it('clears a name a person set when the override is emptied', () => {
		expect(nameToSubmit({ override: '  ', savedName: 'Core Switch', nameSource: 'Manual' })).toBe(
			''
		);
	});

	it('sends a typed override as typed, whatever named the host before', () => {
		expect(nameToSubmit({ override: 'Rack 3', savedName: 'core-sw-01', nameSource: SNMP })).toBe(
			'Rack 3'
		);
		expect(nameToSubmit({ override: 'Rack 3', savedName: '', nameSource: undefined })).toBe(
			'Rack 3'
		);
	});
});
