<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { submitForm } from '$lib/shared/components/forms/form-context';
	import {
		required,
		max,
		port,
		pemCertificate,
		pemPrivateKey,
		sshPrivateKey,
		macAddress,
		ipAddressFormat
	} from '$lib/shared/components/forms/validators';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import type { Credential, CredentialType } from '../types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import { createDefaultCredential } from '../types/base';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { credentialTypes, entities } from '$lib/shared/stores/metadata';
	import { DAEMON_HOST_IP } from '../utils/credentialTargets';
	import { translateFieldDefinitions } from '$lib/i18n/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import type { FieldDefinition, FieldType } from '$lib/shared/stores/metadata';
	import { Eye, EyeOff } from 'lucide-svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import { docsUrl } from '$lib/shared/utils/docs';
	import {
		common_name,
		credentials_credentialType,
		credentials_fileOnHost,
		credentials_filePathReadByDaemon,
		common_enterValue,
		credentials_ipExamplePlaceholder,
		credentials_namePlaceholderExample,
		credentials_secretStoredInDatabase,
		credentials_typeImmutableWarning,
		credentials_docsIntegration,
		credentials_docsIntegrationLinkText,
		daemons_credentialWizardTargetIpHelp,
		daemons_credentialWizardAddRemoteHostTarget,
		daemons_credentialWizardAddDaemonHostTarget,
		daemons_credentialWizardDaemonHostUnavailable,
		daemons_credentialWizardDaemonHostTargetLabel,
		daemons_credentialWizardTargetSpecificHosts,
		daemons_credentialWizardTargetAllHosts,
		daemons_credentialWizardBroadcastHelp,
		credentials_alreadyAssignedTo
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: any;
		credential?: Credential | null;
		fixedCredentialType?: string;
		fixedName?: string;
		compact?: boolean;
		hideFields?: boolean;
		/** Show the name + field inputs but render them disabled (read-only), instead of hiding
		 *  them with `hideFields`. Used for referenced/managed credentials (e.g. a daemon-host
		 *  socket) so the user can see values like socket_path but cannot edit them here. */
		disabled?: boolean;
		/** Hide the target (scope/IP) section — for daemon-host-only types (e.g. the local
		 *  socket) whose target is implicit (127.0.0.1). Shows fields, no target picker. */
		hideTargets?: boolean;
		fieldPrefix?: string;
		/** Disable the "Add daemon host" target when the daemon host is already
		 *  claimed by another single-endpoint credential of the same integration. */
		daemonHostUnavailable?: boolean;
		/** Hosts this credential already reaches through the host/credential junction.
		 *  Listed above the picker so the card reflects everything the credential hits,
		 *  but owned elsewhere — not editable or removable here. */
		lockedHosts?: Host[];
		/** The row's target selection, owned by the parent. This component mirrors it for
		 *  rendering and emits every edit back through `onChange`; it never decides a
		 *  default of its own. Reading these back off the shared TanStack form instead was
		 *  the source of a family of bugs: its defaults are built once, so they are stale
		 *  for any row seeded afterwards. */
		targetIps?: string[];
		scope?: 'broadcast' | 'per_host';
		onChange?: (data: {
			targetIps?: string[];
			fieldValues?: Record<string, string>;
			scope?: 'broadcast' | 'per_host';
			name?: string;
		}) => void;
		onTypeChange?: (typeId: string) => void;
	}

	let {
		form,
		credential = null,
		fixedCredentialType,
		fixedName,
		compact = false,
		hideFields = false,
		disabled = false,
		hideTargets = false,
		fieldPrefix = '',
		daemonHostUnavailable = false,
		lockedHosts = [],
		targetIps,
		scope,
		onChange,
		onTypeChange
	}: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let isEditing = $derived(credential !== null);

	// Selected credential type ID for dynamic form rendering
	let selectedTypeId = $state<string>('SnmpV2c');

	// Notify the parent when the selected type changes (drives the assignments surface)
	$effect(() => {
		onTypeChange?.(selectedTypeId);
	});

	// Dynamic field values keyed by field ID
	let fieldValues = $state<Record<string, string>>({});

	// Where the credential applies: 'per_host' (Hosts — the daemon's own host
	// and/or remote hosts by IP) or 'broadcast' (Networks — all hosts on the
	// network). The available modes and per-host buttons are gated by `targets()`.
	let targetMode = $state<'per_host' | 'broadcast'>('per_host');

	function isDaemonHostValue(value: string): boolean {
		return value === '127.0.0.1' || value === '::1';
	}

	// Which targets the selected type supports (from metadata).
	let supportedTargets = $derived(
		(credentialTypes.getMetadata(selectedTypeId)?.targets ?? []) as string[]
	);
	let supportsNetworks = $derived(supportedTargets.includes('Network'));
	let supportsDaemonHost = $derived(supportedTargets.includes('DaemonHost'));
	let supportsRemoteHosts = $derived(supportedTargets.includes('Hosts'));
	let supportsHosts = $derived(supportsDaemonHost || supportsRemoteHosts);
	// Integration (associated service) name — used in the daemon-host-taken message and the
	// guide link below.
	let integrationName = $derived(
		credentialTypes.getMetadata(selectedTypeId)?.associated_service ?? ''
	);
	// Guide for the selected type's integration. Comes from the credential metadata rather than a
	// branch per type, so every credential type links its guide instead of the two that had one.
	let integrationDocsPath = $derived(credentialTypes.getMetadata(selectedTypeId)?.docs_path ?? '');
	// Show the Hosts | Networks toggle only when both modes are available.
	let showTargetModeToggle = $derived(supportsHosts && supportsNetworks);

	// Get field definitions for the currently selected type (labels/placeholders/
	// help text resolved via meta_* i18n keys with fixture-string fallback)
	let currentFields: FieldDefinition[] = $derived.by(() => {
		const meta = credentialTypes.getMetadata(selectedTypeId);
		return translateFieldDefinitions('credential_types', selectedTypeId, meta?.fields ?? []);
	});

	// Group fields by their group property for visual grouping
	let fieldGroups = $derived.by(() => {
		const groups: { name: string | null; fields: FieldDefinition[] }[] = [];
		const groupOrder: (string | null)[] = [];
		const groupFields: Record<string, FieldDefinition[]> = {};
		const ungroupedFields: FieldDefinition[] = [];

		for (const field of currentFields) {
			const groupName = field.group ?? null;
			if (groupName === null) {
				if (!groupOrder.includes(null)) groupOrder.push(null);
				ungroupedFields.push(field);
			} else {
				if (!groupFields[groupName]) {
					groupFields[groupName] = [];
					groupOrder.push(groupName);
				}
				groupFields[groupName].push(field);
			}
		}

		for (const name of groupOrder) {
			if (name === null) {
				groups.push({ name: null, fields: ungroupedFields });
			} else {
				groups.push({ name, fields: groupFields[name] });
			}
		}
		return groups;
	});

	// Track target IPs as local $state for reactivity (TanStack Form doesn't drive Svelte 5 reactivity)
	let targetIpValues = $state<string[]>(['']);
	let hasDaemonHostTarget = $derived(targetIpValues.some(isDaemonHostValue));

	// --- Secret/file field mode tracking ---
	let secretFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let fileFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let secretFieldVisible = $state<Record<string, boolean>>({});

	function getDefaultValues(): Credential {
		if (credential) return { ...credential };
		if (organization) return createDefaultCredential(organization.id);
		return createDefaultCredential('');
	}

	function initFieldValues(ct: CredentialType) {
		const values: Record<string, string> = {};
		const raw = ct as unknown as Record<string, unknown>;
		const fields = credentialTypes.getMetadata(raw.type as string)?.fields ?? [];
		const fieldMap = new Map(fields.map((f) => [f.id, f]));
		for (const [key, val] of Object.entries(raw)) {
			if (key === 'type') continue;
			const fieldDef = fieldMap.get(key);
			if (
				(fieldDef?.field_type === 'secretpathorinline' ||
					fieldDef?.field_type === 'pathorinline') &&
				val != null &&
				typeof val === 'object'
			) {
				values[key] = JSON.stringify(val);
			} else {
				values[key] = val != null ? String(val) : '';
			}
		}
		fieldValues = values;
		syncFieldsToForm(raw.type as string, 'all');
	}

	function initDefaultFieldValues(typeId: string) {
		const meta = credentialTypes.getMetadata(typeId);
		const fields: FieldDefinition[] = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			if (field.field_type === 'pathorinline') {
				values[field.id] = JSON.stringify({ mode: 'Inline', value: '' });
			} else {
				values[field.id] = field.default_value ?? '';
			}
		}
		fieldValues = values;
		syncFieldsToForm(typeId, 'selects');
	}

	// Fields render from `fieldValues` and only push into the TanStack form on change, so a
	// field the user never touches has no form value at all. Its submit validator still runs
	// and reads `undefined` as empty, so seeding is what lets a value already on screen count
	// as present.
	//
	// `'selects'` covers a new credential: a manually-rendered select must not force the user
	// to re-pick a default it is already showing, but every other field legitimately starts
	// empty and must still fail its required check.
	//
	// `'all'` is for editing an existing credential, where every field arrives pre-filled and a
	// required one the user never touched would otherwise block the save until they retyped the
	// value in front of them (Instant On's Portal Account). Seeding every field is safe only
	// here: the type selector is disabled while editing, so no other credential type's fields
	// can be left behind in the form to fail validation for a field that is no longer on screen.
	function syncFieldsToForm(typeId: string, which: 'all' | 'selects') {
		const fields = credentialTypes.getMetadata(typeId)?.fields ?? [];
		for (const field of fields) {
			if (which === 'selects' && field.field_type !== 'select') continue;
			form.setFieldValue?.(fieldName(field.id), fieldValues[field.id] ?? field.default_value ?? '');
		}
	}

	export function reset() {
		const defaults = getDefaultValues();
		// Only reset form with fields TanStack manages (name). Passing the full
		// credential (with nested credential_type) causes TanStack to register
		// phantom fieldMeta entries for paths like "fields.port" that have no
		// validators, which makes isFieldsValid=false and blocks handleSubmit.
		// eslint-disable-next-line @typescript-eslint/no-unused-vars
		const { credential_type: _ct, ...formFields } = defaults;
		// Only reset the shared form in modal mode. In compact/wizard mode, multiple
		// CredentialForm instances share the same form — resetting it would clear
		// field values set by other instances.
		if (!compact) {
			form.reset(formFields as typeof defaults);
		}
		secretFieldModes = {};
		fileFieldModes = {};
		secretFieldVisible = {};
		targetMode = 'per_host';

		if (credential) {
			selectedTypeId = credential.credential_type.type;
			initFieldValues(credential.credential_type);
			const raw = credential.credential_type as unknown as Record<string, unknown>;
			const fields = credentialTypes.getMetadata(selectedTypeId)?.fields ?? [];
			const fieldMap = new Map(fields.map((f) => [f.id, f]));
			for (const [key, val] of Object.entries(raw)) {
				if (val && typeof val === 'object' && 'mode' in (val as Record<string, unknown>)) {
					const sv = val as { mode: string };
					const mode = sv.mode === 'FilePath' ? 'filepath' : 'inline';
					const fieldDef = fieldMap.get(key);
					if (fieldDef?.field_type === 'pathorinline') {
						fileFieldModes[key] = mode;
					} else {
						secretFieldModes[key] = mode;
					}
				}
			}
			// Reset form fields for modal mode
			if (!compact) {
				form.setFieldValue?.('name', defaults.name);
			}
		} else {
			const typeId = fixedCredentialType ?? 'SnmpV2c';
			selectedTypeId = typeId;
			initDefaultFieldValues(typeId);
		}

		// Set fixed name if provided. In compact mode this seeds this row's slice of the
		// shared form: its defaults are built once, so a row the parent seeds afterwards has
		// no entry and the name input renders blank. (Written inline rather than via
		// `nameFieldName`, which is declared below this function's only call site.)
		if (fixedName) {
			form.setFieldValue?.(compact ? `${fieldPrefix}name` : 'name', fixedName);
		}

		// Mirror the parent's selection. No defaults are chosen here: the row's creator
		// (handleAddCredential / handleAddExistingCredential / the modal's seeding) decides
		// them, so a decision made here could not be emitted and would be silently lost.
		if (compact) {
			targetIpValues = [...(targetIps ?? [])];
			targetMode = scope ?? 'per_host';
			form.setFieldValue?.(`${fieldPrefix}targetIps`, [...targetIpValues]);
		}
	}

	// Initialize on mount (called once, not reactively)
	reset();

	/**
	 * Reset the target selection to the new type's default.
	 *
	 * Reads `targets` inline rather than through the `supportedTargets` derived, which is
	 * still stale at this point in the update (same reason `reset()` computes it inline).
	 * Without this a broadcast mode chosen for a Network-capable type would survive a switch
	 * to one that excludes Network: the toggle hides, but the mode — and the scope it emits —
	 * stays broadcast, producing a target the server discards.
	 */
	function applyDefaultTargetForType(typeId: string) {
		const supported = (credentialTypes.getMetadata(typeId)?.targets ?? []) as string[];
		targetMode = supported.includes('Network') ? 'broadcast' : 'per_host';
		// When the daemon host is the only per-host target, preselect it (the disabled
		// 127.0.0.1 row) so there's nothing for the user to add.
		targetIpValues =
			targetMode === 'per_host' && supported.includes('DaemonHost') && !supported.includes('Hosts')
				? [DAEMON_HOST_IP]
				: [];
		form.setFieldValue?.(`${fieldPrefix}targetIps`, [...targetIpValues]);
		onChange?.({ targetIps: [...targetIpValues], scope: targetMode });
	}

	function handleTypeChange(typeId: string) {
		selectedTypeId = typeId;
		initDefaultFieldValues(selectedTypeId);
		if (compact) applyDefaultTargetForType(selectedTypeId);
	}

	async function handleSubmit() {
		await submitForm(form);
	}

	// Field types whose value is numeric on the wire. `string` is deliberately absent: ports used
	// to be strings that happened to look like numbers, and coercing every string sent a numeric
	// username or site name as a number the server then refused.
	function submitsAsNumber(fieldType: FieldType): boolean {
		return fieldType === 'number' || fieldType === 'port';
	}

	/** Build a CredentialType from current fieldValues. */
	export function buildCredentialType(): CredentialType {
		const fields = currentFields;
		const typeObj: Record<string, unknown> = { type: selectedTypeId };

		for (const field of fields) {
			const value = fieldValues[field.id];
			if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
				if (field.optional && (!value || value.trim() === '')) {
					typeObj[field.id] = null;
				} else {
					try {
						const parsed = JSON.parse(value);
						// Normalize empty inline/path values to null for optional fields
						if (
							field.optional &&
							((parsed.mode === 'Inline' && !parsed.value?.trim()) ||
								(parsed.mode === 'FilePath' && !parsed.path?.trim()))
						) {
							typeObj[field.id] = null;
						} else {
							typeObj[field.id] = parsed;
						}
					} catch {
						typeObj[field.id] = { mode: 'Inline', value };
					}
				}
			} else if (field.optional && (!value || value.trim() === '')) {
				if (field.default_value != null) {
					const dv = field.default_value;
					const dvNum = Number(dv);
					typeObj[field.id] =
						dv !== '' && !isNaN(dvNum) && submitsAsNumber(field.field_type) ? dvNum : dv;
				} else {
					typeObj[field.id] = null;
				}
			} else {
				const raw = value ?? (field.default_value || '');
				const num = Number(raw);
				typeObj[field.id] =
					raw !== '' && !isNaN(num) && submitsAsNumber(field.field_type) ? num : raw;
			}
		}

		return typeObj as unknown as CredentialType;
	}

	// All credential types are user-selectable (sockets included, created like any other).
	let typeOptions = $derived(credentialTypes.getItems());

	// Whether to show type selector and name field
	let showTypeSelector = $derived(!fixedCredentialType);
	let showName = $derived(!fixedName && !compact);

	// --- Field name helpers ---
	function fieldName(id: string): string {
		return `${fieldPrefix}fields.${id}`;
	}

	function targetIpFieldName(index: number): string {
		return `${fieldPrefix}targetIps[${index}]`;
	}

	let nameFieldName = $derived(`${fieldPrefix}name`);

	// --- Secret/file field helpers ---
	function getSecretFieldMode(fieldId: string): 'inline' | 'filepath' {
		return secretFieldModes[fieldId] ?? 'inline';
	}

	function setSecretFieldMode(fieldId: string, mode: 'inline' | 'filepath') {
		secretFieldModes[fieldId] = mode;
		const current = fieldValues[fieldId];
		let parsed: { mode?: string; value?: string; path?: string };
		try {
			parsed = current ? JSON.parse(current) : {};
		} catch {
			parsed = {};
		}
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'Inline',
				value: parsed.value ?? parsed.path ?? ''
			});
		} else {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'FilePath',
				path: parsed.path ?? parsed.value ?? ''
			});
		}
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function getSecretFieldDisplayValue(fieldId: string): string {
		const raw = fieldValues[fieldId];
		if (!raw) return '';
		try {
			const parsed = JSON.parse(raw);
			if (parsed.mode === 'Inline') return parsed.value ?? '';
			if (parsed.mode === 'FilePath') return parsed.path ?? '';
		} catch {
			// not JSON yet
		}
		return raw;
	}

	function setSecretFieldDisplayValue(fieldId: string, displayValue: string) {
		const mode = getSecretFieldMode(fieldId);
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({ mode: 'Inline', value: displayValue });
		} else {
			fieldValues[fieldId] = JSON.stringify({ mode: 'FilePath', path: displayValue });
		}
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function getFileFieldMode(fieldId: string): 'inline' | 'filepath' {
		return fileFieldModes[fieldId] ?? 'inline';
	}

	function setFileFieldMode(fieldId: string, mode: 'inline' | 'filepath') {
		fileFieldModes[fieldId] = mode;
		const current = fieldValues[fieldId];
		let parsed: { mode?: string; value?: string; path?: string };
		try {
			parsed = current ? JSON.parse(current) : {};
		} catch {
			parsed = {};
		}
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'Inline',
				value: parsed.value ?? parsed.path ?? ''
			});
		} else {
			fieldValues[fieldId] = JSON.stringify({
				mode: 'FilePath',
				path: parsed.path ?? parsed.value ?? ''
			});
		}
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function getFileFieldDisplayValue(fieldId: string): string {
		const raw = fieldValues[fieldId];
		if (!raw) return '';
		try {
			const parsed = JSON.parse(raw);
			if (parsed.mode === 'Inline') return parsed.value ?? '';
			if (parsed.mode === 'FilePath') return parsed.path ?? '';
		} catch {
			// not JSON yet
		}
		return raw;
	}

	function setFileFieldDisplayValue(fieldId: string, displayValue: string) {
		const mode = getFileFieldMode(fieldId);
		if (mode === 'inline') {
			fieldValues[fieldId] = JSON.stringify({ mode: 'Inline', value: displayValue });
		} else {
			fieldValues[fieldId] = JSON.stringify({ mode: 'FilePath', path: displayValue });
		}
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function syncTargets() {
		const next = [...targetIpValues];
		form.setFieldValue?.(`${fieldPrefix}targetIps`, next);
		onChange?.({ targetIps: next, scope: 'per_host' });
	}

	function handleTargetModeChange(mode: 'per_host' | 'broadcast') {
		targetMode = mode;
		if (mode === 'broadcast') {
			targetIpValues = [];
			form.setFieldValue?.(`${fieldPrefix}targetIps`, []);
			onChange?.({ targetIps: [], scope: 'broadcast' });
		} else {
			// Hosts: leave the target list as-is (empty until the user adds one)
			syncTargets();
		}
	}

	/**
	 * Validate the target selection (compact wizard). "Target Specific Hosts"
	 * requires at least one host (a remote IP or the daemon-host row). Broadcast is
	 * always valid. The caller surfaces failures via a toast on advance.
	 */
	/**
	 * The human label for one of this form's field paths, for the validation toast.
	 *
	 * Without it `validateForm` falls back to the raw path and names something the form never
	 * showed: the Instant On portal login is `fields.username` but is labelled Portal Account.
	 */
	export function fieldLabel(fieldPath: string): string {
		const prefix = `${fieldPrefix}fields.`;
		if (!fieldPath.startsWith(prefix)) return fieldPath.replace(/_/g, ' ');
		const id = fieldPath.slice(prefix.length);
		const fields = credentialTypes.getMetadata(selectedTypeId)?.fields ?? [];
		return fields.find((f) => f.id === id)?.label ?? id.replace(/_/g, ' ');
	}

	export function validateTarget(): boolean {
		// A hidden picker can never be a failure the user could act on: `hideTargets` types
		// (the daemon-host socket, managed references) carry an implicit 127.0.0.1 target.
		if (!compact || hideTargets || targetMode === 'broadcast') return true;
		// Locked rows are real targets owned elsewhere, so the credential is already
		// pointed somewhere even with nothing entered here.
		if (lockedHosts.length > 0) return true;
		return targetIpValues.some((ip) => (ip?.trim() ?? '') !== '');
	}

	// Target-IP field validator. Empty rows are valid at the field level — they're
	// dropped on save, and the "at least one target" rule is enforced by
	// `validateTarget()`. This avoids a stale empty row (e.g. added then removed)
	// failing field validation. Only the IP format of non-empty rows is checked.
	// Broadcast credentials always pass (stale targetIps fields left after toggling
	// Hosts -> Networks can't block submission). Reads the live, reactive `targetMode`.
	// `value` can be undefined: TanStack keeps a field registered after its row is removed,
	// and `validateAllFields` on submit then runs it against an index the array no longer
	// has. An absent value is an empty row, which is valid here.
	function validateTargetIp(value: string | undefined): string | undefined {
		if (targetMode === 'broadcast' || !value?.trim()) return undefined;
		return ipAddressFormat(value);
	}

	function handleAddIpTarget() {
		targetIpValues = [...targetIpValues, ''];
		syncTargets();
	}

	function handleAddDaemonHostTarget() {
		if (hasDaemonHostTarget) return;
		targetIpValues = [...targetIpValues, DAEMON_HOST_IP];
		syncTargets();
	}

	function handleFieldValueChange(fieldId: string, value: string) {
		fieldValues[fieldId] = value;
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function handleTargetIpChange(index: number, value: string) {
		targetIpValues[index] = value;
		targetIpValues = [...targetIpValues];
		const formValues = [...targetIpValues];
		form.setFieldValue?.(`${fieldPrefix}targetIps`, formValues);
		// Emits the mode too: every edit reports the complete selection, so the parent's
		// copy can never drift from what is rendered here.
		onChange?.({ targetIps: formValues, scope: targetMode });
	}

	function handleRemoveTarget(index: number) {
		targetIpValues = targetIpValues.filter((_, i) => i !== index);
		syncTargets();
	}

	// Build validators for a credential field based on its definition
	function getFieldValidators(field: FieldDefinition) {
		const validate = ({ value }: { value: string }) => {
			// For path-or-inline fields, check the actual display value, not the JSON wrapper
			let effectiveValue = value;
			if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
				if (field.field_type === 'secretpathorinline') {
					effectiveValue = getSecretFieldDisplayValue(field.id);
				} else {
					effectiveValue = getFileFieldDisplayValue(field.id);
				}
			}
			if (!field.optional && !effectiveValue?.trim()) return 'This field is required';
			// Skip all further validation if value is empty (optional field)
			if (!effectiveValue?.trim()) return undefined;
			if (field.field_type === 'port') {
				return port(effectiveValue);
			}
			// Only validate PEM format when in inline mode
			if (field.field_type === 'secretpathorinline') {
				if (getSecretFieldMode(field.id) !== 'inline') return undefined;
			}
			if (field.field_type === 'pathorinline') {
				if (getFileFieldMode(field.id) !== 'inline') return undefined;
			}
			if (field.inline_format === 'pemprivatekey' && effectiveValue !== '********') {
				return pemPrivateKey(effectiveValue);
			}
			if (field.inline_format === 'pemcertificate') {
				return pemCertificate(effectiveValue);
			}
			if (field.inline_format === 'sshprivatekey' && effectiveValue !== '********') {
				return sshPrivateKey(effectiveValue);
			}
			if (field.inline_format === 'macaddress' && effectiveValue !== '********') {
				return macAddress(effectiveValue);
			}
			return undefined;
		};
		return { onBlur: validate, onSubmit: validate };
	}
</script>

{#if compact}
	<div class="space-y-4">
		{#if !hideTargets}
			<!-- Hosts this credential already reaches through the host/credential junction.
			     Informational, like the network-wide credential line in the wizard — these are
			     owned elsewhere, so they are listed rather than offered as editable rows.
			     Shown in both modes: it is a fact about the credential, not about the choice
			     being made here. -->
			{#if lockedHosts.length > 0}
				<p class="text-tertiary flex flex-wrap items-center gap-1 text-xs">
					<span>{credentials_alreadyAssignedTo()}</span>
					{#each lockedHosts as host (host.id)}
						<EntityTag
							entityRef={entityRef('Host', host.id, host)}
							label={hostDisplayName(host)}
							icon={entities.getIconComponent('Host')}
							color={entities.getColorHelper('Host').color}
						/>
					{/each}
				</p>
			{/if}

			<!-- Target mode selector — only when the type supports both modes -->
			{#if showTargetModeToggle}
				<SegmentedControl
					options={[
						{ value: 'per_host', label: daemons_credentialWizardTargetSpecificHosts() },
						{ value: 'broadcast', label: daemons_credentialWizardTargetAllHosts() }
					]}
					selected={targetMode}
					onchange={(v) => handleTargetModeChange(v as 'per_host' | 'broadcast')}
					size="sm"
				/>
			{/if}

			{#if targetMode === 'broadcast'}
				<p class="text-muted text-xs">{daemons_credentialWizardBroadcastHelp()}</p>
			{:else}
				{#each targetIpValues as ip, i (i)}
					<div class="flex items-center gap-2">
						{#if isDaemonHostValue(ip)}
							<input
								type="text"
								class="input-field min-w-0 flex-1"
								value={daemons_credentialWizardDaemonHostTargetLabel()}
								disabled
							/>
						{:else}
							<div class="min-w-0 flex-1">
								<form.Field
									name={targetIpFieldName(i)}
									validators={{
										onBlur: ({ value }: { value: string | undefined }) => validateTargetIp(value),
										onChange: ({ value }: { value: string | undefined }) => validateTargetIp(value),
										onSubmit: ({ value }: { value: string | undefined }) => validateTargetIp(value)
									}}
									listeners={{
										onChange: ({ value }: { value: string }) => handleTargetIpChange(i, value)
									}}
								>
									{#snippet children(field: AnyFieldApi)}
										<TextInput
											label=""
											id="target-ip-{fieldPrefix}{i}"
											placeholder={credentials_ipExamplePlaceholder()}
											{field}
										/>
									{/snippet}
								</form.Field>
							</div>
						{/if}
						<button
							type="button"
							class="text-muted hover:text-primary shrink-0 p-1 text-lg leading-none"
							onclick={() => handleRemoveTarget(i)}>&times;</button
						>
					</div>
				{/each}
				<div class="flex flex-wrap items-center gap-3">
					{#if supportsDaemonHost}
						{#if daemonHostUnavailable && !hasDaemonHostTarget}
							<!-- Claimed by another credential: unselectable, reason on hover -->
							<span
								class="inline-block"
								data-tooltip={daemons_credentialWizardDaemonHostUnavailable({
									integration: integrationName
								})}
								use:tooltip
							>
								<button
									type="button"
									class="text-muted cursor-not-allowed text-sm opacity-40"
									disabled>+ {daemons_credentialWizardAddDaemonHostTarget()}</button
								>
							</span>
						{:else if hasDaemonHostTarget}
							<!-- Already added: disabled, no hover state -->
							<button
								type="button"
								class="text-muted cursor-not-allowed text-sm opacity-40"
								disabled>+ {daemons_credentialWizardAddDaemonHostTarget()}</button
							>
						{:else}
							<button type="button" class="text-link text-sm" onclick={handleAddDaemonHostTarget}
								>+ {daemons_credentialWizardAddDaemonHostTarget()}</button
							>
						{/if}
					{/if}
					{#if supportsRemoteHosts}
						<button type="button" class="text-link text-sm" onclick={handleAddIpTarget}
							>+ {daemons_credentialWizardAddRemoteHostTarget()}</button
						>
					{/if}
				</div>
				<p class="text-muted text-xs">{daemons_credentialWizardTargetIpHelp()}</p>
			{/if}
		{/if}

		<!-- Name + fields. `disabled` shows them read-only (vs `hideFields` which hides them);
		     a disabled <fieldset> disables every descendant control, including SegmentedControls. -->
		{#if !hideFields}
			<fieldset {disabled} class="m-0 min-w-0 space-y-4 border-0 p-0">
				<form.Field
					name={nameFieldName}
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value),
						onSubmit: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
					listeners={{ onChange: ({ value }: { value: string }) => onChange?.({ name: value }) }}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_name()}
							id="credential-name-{fieldPrefix}"
							{field}
							{disabled}
							required
						/>
					{/snippet}
				</form.Field>

				{#each fieldGroups as group (group.name ?? '_ungrouped')}
					{#if group.name}
						<InfoCard title={group.name}>
							{#each group.fields as field (field.id)}
								{@render fieldRenderer(field, field.secret)}
							{/each}
						</InfoCard>
					{:else if group.fields.length > 0}
						<InfoCard title={null}>
							{#each group.fields as field (field.id)}
								{@render fieldRenderer(field, field.secret)}
							{/each}
						</InfoCard>
					{/if}
				{/each}
			</fieldset>
		{/if}
	</div>
{:else}
	<form
		onsubmit={(e) => {
			e.preventDefault();
			e.stopPropagation();
			handleSubmit();
		}}
		class="flex flex-col gap-4"
	>
		<!-- Standard mode: card wrapper for name/type, separate cards for fields -->
		<div class="card card-static space-y-4 p-4">
			{#if showName}
				<form.Field
					name={nameFieldName}
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value),
						onSubmit: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_name()}
							id="credential-name"
							{field}
							placeholder={credentials_namePlaceholderExample()}
							required
						/>
					{/snippet}
				</form.Field>
			{/if}

			{#if showTypeSelector}
				<div class="space-y-2">
					<RichSelect
						label={credentials_credentialType()}
						selectedValue={selectedTypeId}
						options={typeOptions}
						displayComponent={CredentialTypeDisplay}
						disabled={isEditing}
						onSelect={handleTypeChange}
					/>
					{#if !isEditing}
						<p class="text-muted mt-1 text-xs">{credentials_typeImmutableWarning()}</p>
					{/if}
				</div>
			{/if}

			{#if integrationDocsPath}
				<DocsHint
					text={credentials_docsIntegration()}
					href={docsUrl(integrationDocsPath)}
					linkText={credentials_docsIntegrationLinkText({ integration: integrationName })}
				/>
			{/if}
		</div>

		{#each fieldGroups as group (group.name ?? '_ungrouped')}
			{#if group.name}
				<InfoCard title={group.name}>
					{#each group.fields as field (field.id)}
						{@render fieldRenderer(field, field.secret)}
					{/each}
				</InfoCard>
			{:else if group.fields.length > 0}
				<div class="card card-static space-y-4 p-4">
					{#each group.fields as field (field.id)}
						{@render fieldRenderer(field, field.secret)}
					{/each}
				</div>
			{/if}
		{/each}

		<!-- Hidden submit button for Enter-to-submit -->
		<button type="submit" class="hidden" aria-hidden="true" tabindex={-1}></button>
	</form>
{/if}

{#snippet fieldRenderer(field: FieldDefinition, isSecret: boolean)}
	{@const fName = fieldName(field.id)}
	<div class="space-y-1">
		{#if field.field_type === 'select'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<select
						id={field.id}
						value={fieldValues[field.id] ?? field.default_value ?? ''}
						onchange={(e) => {
							const target = e.target as HTMLSelectElement;
							handleFieldValueChange(field.id, target.value);
							formField.handleChange(target.value);
						}}
						onblur={() => formField.handleBlur()}
						class="select-trigger text-primary w-full rounded-md px-3 py-2 text-sm"
						class:input-field-error={formField.state.meta.errors?.length > 0}
					>
						{#each field.options ?? [] as option (option.value)}
							<option value={option.value}>{option.label}</option>
						{/each}
					</select>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'secretpathorinline'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<div class="space-y-2">
						<SegmentedControl
							options={[
								{ value: 'inline', label: common_enterValue() },
								{ value: 'filepath', label: credentials_fileOnHost() }
							]}
							selected={getSecretFieldMode(field.id)}
							onchange={(v) => setSecretFieldMode(field.id, v as 'inline' | 'filepath')}
							size="sm"
						/>
						{#if getSecretFieldMode(field.id) === 'inline'}
							<p class="text-muted text-xs">
								{credentials_secretStoredInDatabase()}
							</p>
							{#if field.inline_format === 'pemprivatekey' || field.inline_format === 'sshprivatekey' || !field.inline_format}
								<div class="relative">
									<textarea
										id={field.id}
										value={getSecretFieldDisplayValue(field.id)}
										oninput={(e) => {
											const target = e.target as HTMLTextAreaElement;
											setSecretFieldDisplayValue(field.id, target.value);
											formField.handleChange(target.value);
										}}
										onblur={() => formField.handleBlur()}
										placeholder={field.placeholder ?? '-----BEGIN PRIVATE KEY-----'}
										rows={4}
										class="input-field text-primary w-full rounded-md px-3 py-2 pr-10 font-mono text-sm"
										class:password-field={!secretFieldVisible[field.id]}
										class:input-field-error={formField.state.meta.errors?.length > 0}
									></textarea>
									{#if getSecretFieldDisplayValue(field.id) && getSecretFieldDisplayValue(field.id) !== '********'}
										<button
											type="button"
											class="text-muted hover:text-secondary absolute right-2 top-2"
											onclick={() => (secretFieldVisible[field.id] = !secretFieldVisible[field.id])}
										>
											{#if secretFieldVisible[field.id]}
												<EyeOff class="h-4 w-4" />
											{:else}
												<Eye class="h-4 w-4" />
											{/if}
										</button>
									{/if}
								</div>
							{:else}
								<div class="relative">
									<input
										id={field.id}
										type={secretFieldVisible[field.id] ? 'text' : 'password'}
										value={getSecretFieldDisplayValue(field.id)}
										oninput={(e) => {
											const target = e.target as HTMLInputElement;
											setSecretFieldDisplayValue(field.id, target.value);
											formField.handleChange(target.value);
										}}
										onblur={() => formField.handleBlur()}
										placeholder={field.placeholder ?? ''}
										class="input-field text-primary w-full rounded-md px-3 py-2 pr-10 text-sm"
										class:input-field-error={formField.state.meta.errors?.length > 0}
									/>
									{#if getSecretFieldDisplayValue(field.id) && getSecretFieldDisplayValue(field.id) !== '********'}
										<button
											type="button"
											class="text-muted hover:text-secondary absolute right-2 top-1/2 -translate-y-1/2"
											onclick={() => (secretFieldVisible[field.id] = !secretFieldVisible[field.id])}
										>
											{#if secretFieldVisible[field.id]}
												<EyeOff class="h-4 w-4" />
											{:else}
												<Eye class="h-4 w-4" />
											{/if}
										</button>
									{/if}
								</div>
							{/if}
						{:else}
							<p class="text-muted text-xs">
								{credentials_filePathReadByDaemon()}
							</p>
							<input
								id={field.id}
								type="text"
								value={getSecretFieldDisplayValue(field.id)}
								oninput={(e) => {
									const target = e.target as HTMLInputElement;
									setSecretFieldDisplayValue(field.id, target.value);
									formField.handleChange(target.value);
								}}
								onblur={() => formField.handleBlur()}
								placeholder={field.inline_format === 'pemprivatekey' ||
								field.inline_format === 'sshprivatekey'
									? '/path/to/key.pem'
									: '/path/to/secret'}
								class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'pathorinline'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<div class="space-y-2">
						<SegmentedControl
							options={[
								{ value: 'inline', label: common_enterValue() },
								{ value: 'filepath', label: credentials_fileOnHost() }
							]}
							selected={getFileFieldMode(field.id)}
							onchange={(v) => setFileFieldMode(field.id, v as 'inline' | 'filepath')}
							size="sm"
						/>
						{#if getFileFieldMode(field.id) === 'inline'}
							<p class="text-muted text-xs">
								{credentials_secretStoredInDatabase()}
							</p>
							{#if field.inline_format === 'pemcertificate'}
								<textarea
									id={field.id}
									value={getFileFieldDisplayValue(field.id)}
									oninput={(e) => {
										const target = e.target as HTMLTextAreaElement;
										setFileFieldDisplayValue(field.id, target.value);
										formField.handleChange(target.value);
									}}
									onblur={() => formField.handleBlur()}
									placeholder={field.placeholder ?? ''}
									rows={4}
									class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
									class:input-field-error={formField.state.meta.errors?.length > 0}
								></textarea>
							{:else}
								<input
									id={field.id}
									type="text"
									value={getFileFieldDisplayValue(field.id)}
									oninput={(e) => {
										const target = e.target as HTMLInputElement;
										setFileFieldDisplayValue(field.id, target.value);
										formField.handleChange(target.value);
									}}
									onblur={() => formField.handleBlur()}
									placeholder={field.placeholder ?? ''}
									class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
									class:input-field-error={formField.state.meta.errors?.length > 0}
								/>
							{/if}
						{:else}
							<p class="text-muted text-xs">
								{credentials_filePathReadByDaemon()}
							</p>
							<input
								id={field.id}
								type="text"
								value={getFileFieldDisplayValue(field.id)}
								oninput={(e) => {
									const target = e.target as HTMLInputElement;
									setFileFieldDisplayValue(field.id, target.value);
									formField.handleChange(target.value);
								}}
								onblur={() => formField.handleBlur()}
								placeholder="/etc/docker/certs/cert.pem"
								class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'text'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<textarea
						id={field.id}
						value={fieldValues[field.id] ?? ''}
						oninput={(e) => {
							const target = e.target as HTMLTextAreaElement;
							handleFieldValueChange(field.id, target.value);
							formField.handleChange(target.value);
						}}
						onblur={() => formField.handleBlur()}
						placeholder={field.placeholder ?? ''}
						rows={8}
						class="input-field text-primary w-full rounded-md px-3 py-2 font-mono text-sm"
						class:password-field={isSecret}
						class:input-field-error={formField.state.meta.errors?.length > 0}
					></textarea>
				{/snippet}
			</form.Field>
		{:else}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="text-secondary block text-sm font-medium">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<input
						id={field.id}
						type={isSecret ? 'password' : 'text'}
						value={fieldValues[field.id] ?? ''}
						oninput={(e) => {
							const target = e.target as HTMLInputElement;
							handleFieldValueChange(field.id, target.value);
							formField.handleChange(target.value);
						}}
						onblur={() => formField.handleBlur()}
						placeholder={field.placeholder ?? ''}
						class="input-field text-primary w-full rounded-md px-3 py-2 text-sm"
						class:input-field-error={formField.state.meta.errors?.length > 0}
					/>
				{/snippet}
			</form.Field>
		{/if}

		{#if field.help_text}
			<p class="text-muted text-xs">{field.help_text}</p>
		{/if}
	</div>
{/snippet}

<style>
	.password-field {
		-webkit-text-security: disc;
	}
</style>
