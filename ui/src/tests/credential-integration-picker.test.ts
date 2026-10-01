import { describe, it, expect } from 'vitest';
import {
	groupTypesByIntegration,
	initiallyExpandedIntegrationIds,
	keepPendingRow,
	pickerSelectionFromPending,
	selectedTypeCount,
	type PickerPendingRow,
	type PickerType
} from '$lib/features/credentials/utils/integrationPicker';
import credentialTypesJson from '../lib/data/credential-types.json';
import credentialIntegrationsJson from '../lib/data/credential-integrations.json';

const types = credentialTypesJson as PickerType[];
const integrations = credentialIntegrationsJson as { id: string }[];
const groups = groupTypesByIntegration(types, integrations);

const multiGroup = groups.find((g) => g.types.length > 1)!;
const singleGroup = groups.find((g) => g.types.length === 1)!;

function row(type: string, id: string, extra: Partial<PickerPendingRow> = {}): PickerPendingRow {
	return { credential: { id, credential_type: { type } }, ...extra };
}

describe('groupTypesByIntegration', () => {
	it('places every credential type in exactly one integration group', () => {
		const grouped = groups.flatMap((g) => g.types.map((t) => t.id));
		expect(grouped.sort()).toEqual(types.map((t) => t.id).sort());
	});

	it('gives each fixture integration one row', () => {
		expect(groups.map((g) => g.integration.id).sort()).toEqual(
			integrations.map((i) => i.id).sort()
		);
	});

	it('includes both single-type and multi-type integrations', () => {
		expect(multiGroup).toBeDefined();
		expect(singleGroup).toBeDefined();
	});

	it('orders daemon-only integrations before network-applicable ones', () => {
		const reach = (t: PickerType) => {
			const targets = t.metadata?.targets ?? [];
			return targets.includes('Network') ? 2 : targets.includes('Hosts') ? 1 : 0;
		};
		const groupReach = groups.map((g) => Math.min(...g.types.map(reach)));
		expect(groupReach).toEqual([...groupReach].sort((a, b) => a - b));
		for (const g of groups) {
			const typeReach = g.types.map(reach);
			expect(typeReach).toEqual([...typeReach].sort((a, b) => a - b));
		}
	});

	it('leaves out a type whose integration has no fixture entry', () => {
		const orphan: PickerType = { id: 'Orphan', metadata: { integration: 'Missing' } };
		const result = groupTypesByIntegration([...types, orphan], integrations);
		expect(result.flatMap((g) => g.types).some((t) => t.id === 'Orphan')).toBe(false);
	});
});

describe('selectedTypeCount', () => {
	it('counts only the group’s own checked types', () => {
		const [first] = multiGroup.types;
		expect(selectedTypeCount(multiGroup, [first.id, singleGroup.types[0].id])).toBe(1);
		expect(
			selectedTypeCount(
				multiGroup,
				multiGroup.types.map((t) => t.id)
			)
		).toBe(multiGroup.types.length);
		expect(selectedTypeCount(multiGroup, [])).toBe(0);
	});
});

describe('initiallyExpandedIntegrationIds', () => {
	it('expands a multi-type integration holding a selection', () => {
		const ids = initiallyExpandedIntegrationIds(groups, [multiGroup.types[0].id]);
		expect(ids).toEqual([multiGroup.integration.id]);
	});

	it('never expands a single-type integration, even when selected', () => {
		expect(initiallyExpandedIntegrationIds(groups, [singleGroup.types[0].id])).toEqual([]);
	});

	it('starts everything collapsed with nothing selected', () => {
		expect(initiallyExpandedIntegrationIds(groups, [])).toEqual([]);
	});
});

describe('returning from the wizard to the picker', () => {
	const rows = [
		row('SnmpV3', 'new-snmp'),
		row('SshKey', 'saved-ssh'),
		row('DockerProxy', 'existing-docker', { isExisting: true }),
		row('SnmpV2c', 'assigned-elsewhere', { isExisting: true, lockedHosts: [{}] })
	];
	const saved = ['saved-ssh'];

	it('selects only the types of new, unsaved rows', () => {
		expect(pickerSelectionFromPending(rows, saved)).toEqual(['SnmpV3']);
	});

	it('drops a new row whose type was deselected and keeps every other row', () => {
		const kept = rows.filter((r) => keepPendingRow(r, [], saved)).map((r) => r.credential.id);
		expect(kept).toEqual(['saved-ssh', 'existing-docker', 'assigned-elsewhere']);
	});

	it('keeps a new row whose type stays selected', () => {
		expect(keepPendingRow(rows[0], ['SnmpV3'], saved)).toBe(true);
	});
});
