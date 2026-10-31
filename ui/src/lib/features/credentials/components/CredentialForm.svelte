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
		proxmoxTokenId,
		ipAddressFormat
	} from '$lib/shared/components/forms/validators';
	import SegmentedControl from '$lib/shared/components/forms/SegmentedControl.svelte';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { CredentialTypeDisplay } from '$lib/shared/components/forms/selection/display/CredentialTypeDisplay.svelte';
	import type { Credential, CredentialType } from '../types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';
	import {
		createDefaultCredential,
		credentialTypeUpgradeFeature,
		requiresLogin
	} from '../types/base';
	import LoginPicker from './LoginPicker.svelte';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import { credentialTypes, entities } from '$lib/shared/stores/metadata';
	import { DAEMON_HOST_IP } from '../utils/credentialTargets';
	import { fieldRows } from '../utils/fieldRows';
	import { shownFields } from '../utils/fieldSets';
	import { translateFieldDefinitions } from '$lib/i18n/metadata';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import RadioGroup from '$lib/shared/components/forms/input/RadioGroup.svelte';
	import type { FieldDefinition, FieldType } from '$lib/shared/stores/metadata';
	import { Eye, EyeOff, HelpCircle, Plus } from '@lucide/svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import { docsUrl } from '$lib/shared/utils/docs';
	import {
		DAEMON_OS_FIELD,
		filePathPlaceholder,
		osFamilyOptions,
		resolvePlaceholder,
		type OsFamily
	} from '../utils/placeholders';
	import {
		credentialFieldValues,
		defaultFieldValue,
		parseScriptSource,
		scriptSourceText,
		withScriptSourceMode,
		withScriptSourceText,
		type ScriptSource,
		type ScriptSourceMode
	} from '../utils/fieldValues';
	import {
		common_description,
		common_details,
		common_name,
		common_stepNumber,
		daemons_credentialWizardStepConfigure,
		daemons_credentialWizardStepSelectTarget,
		common_options,
		credentials_credentialType,
		credentials_daemonOs,
		credentials_daemonOsHelp,
		credentials_fileOnDaemonHost,
		credentials_fileOnScannedHost,
		credentials_filePathReadByDaemon,
		credentials_filePathRunByScannedHost,
		credentials_scriptOutputDocs,
		credentials_scriptOutputDocsLinkText,
		common_enterValue,
		credentials_ipExamplePlaceholder,
		credentials_descriptionPlaceholder,
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
		daemons_credentialWizardDaemonHostTargetFixed,
		credentials_alreadyAssignedTo,
		credentials_sshLoginRequired
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: any;
		credential?: Credential | null;
		fixedCredentialType?: string;
		/** Limit the type picker to these types and pre-select the first, for a create flow opened
		 *  for one integration. Ignored when editing or when `fixedCredentialType` is set. */
		typeChoices?: string[];
		/** Why a type can't be picked here, or null. Shown as the option's tooltip. */
		typeDisabledReason?: (typeId: string) => string | null;
		fixedName?: string;
		compact?: boolean;
		hideFields?: boolean;
		/** Shown in Step 2 in place of the name + field inputs. Used for a saved credential added
		 *  by reference, whose fields are edited from the credential itself, not here. */
		fieldsNotice?: string;
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
		/** The daemon OS when the caller already knows it, e.g. the create-daemon flow, which asks
		 *  for the OS first. Overrides the credential's own and hides the Daemon OS picker. */
		fixedDaemonOs?: OsFamily | null;
		/** Standard mode only: which half to show. `identity` is name, description and type;
		 *  `fields` is the type's own fields. The other half's fields unmount (so a step's
		 *  validation reaches only what is on screen); this component stays mounted, so values and
		 *  field modes survive stepping between them. */
		section?: 'all' | 'identity' | 'fields';
		/** Standard mode: what Enter does. A caller that steps through the form (the create flow)
		 *  passes its Next; without one, Enter submits. */
		onSubmitRequest?: () => void;
	}

	let {
		form,
		credential = null,
		fixedCredentialType,
		typeChoices,
		typeDisabledReason,
		fixedName,
		compact = false,
		hideFields = false,
		fieldsNotice,
		hideTargets = false,
		fieldPrefix = '',
		daemonHostUnavailable = false,
		lockedHosts = [],
		targetIps,
		scope,
		onChange,
		onTypeChange,
		fixedDaemonOs = null,
		section = 'all',
		onSubmitRequest
	}: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let isEditing = $derived(credential !== null);

	// Selected credential type ID for dynamic form rendering
	let selectedTypeId = $state<string>('SnmpCommunity');

	// Notify the parent when the selected type changes (drives the assignments surface)
	$effect(() => {
		onTypeChange?.(selectedTypeId);
	});

	// Dynamic field values keyed by field ID
	let fieldValues = $state<Record<string, string>>({});

	// Where the credential applies: 'per_host' (Hosts — the daemon's own host
	// and/or remote hosts by IP) or 'broadcast' (Sites — all hosts on the
	// site). The available modes and per-host buttons are gated by `targets()`.
	let targetMode = $state<'per_host' | 'broadcast'>('per_host');

	function isDaemonHostValue(value: string): boolean {
		return value === '127.0.0.1' || value === '::1';
	}

	// Which targets the selected type supports (from metadata).
	let supportedTargets = $derived(
		(credentialTypes.getMetadata(selectedTypeId)?.targets ?? []) as string[]
	);
	let supportsSites = $derived(supportedTargets.includes('Site'));
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
	// Show the Hosts | Sites toggle only when both modes are available.
	let showTargetModeToggle = $derived(supportsHosts && supportsSites);

	// Get field definitions for the currently selected type (labels/placeholders/
	// help text resolved via meta_* i18n keys with fixture-string fallback)
	let typeFields: FieldDefinition[] = $derived.by(() => {
		const meta = credentialTypes.getMetadata(selectedTypeId);
		return translateFieldDefinitions('credential_types', selectedTypeId, meta?.fields ?? []);
	});

	// The fields the current values call for: a field-set dropdown (SNMPv3's security level) hides
	// the fields its other values use. Hidden fields are neither rendered, validated nor sent, so a
	// credential never carries a field its own choice does not use. Reads `fieldValues`, the
	// reactive source of truth, not TanStack's untracked form state.
	let currentFields: FieldDefinition[] = $derived(shownFields(typeFields, fieldValues));

	// Group fields by their group property for visual grouping
	function groupFieldsByGroup(fields: FieldDefinition[]) {
		const groups: { name: string | null; fields: FieldDefinition[] }[] = [];
		const groupOrder: (string | null)[] = [];
		const groupFields: Record<string, FieldDefinition[]> = {};
		const ungroupedFields: FieldDefinition[] = [];

		for (const field of fields) {
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
		return groups.filter((g) => g.fields.length > 0);
	}

	let visibleFieldGroups = $derived(groupFieldsByGroup(currentFields));

	type FieldGroup = ReturnType<typeof groupFieldsByGroup>[number];
	interface FieldCard {
		key: string;
		title: string;
		/** Holds the name/description/type fields (standard mode only). */
		identity: boolean;
		groups: FieldGroup[];
	}

	// Ungrouped fields (e.g. SNMPv3's context name) need a title once they sit beside named groups.
	function groupTitle(name: string | null): string {
		return name ?? common_options();
	}

	// A card holds two or more fields: a group with one field joins the card before it (the
	// Details card for the first group), or the next card when nothing precedes it.
	function buildFieldCards(withIdentity: boolean, groups: FieldGroup[]): FieldCard[] {
		const cards: FieldCard[] = [];
		if (withIdentity) {
			cards.push({ key: '_identity', title: common_details(), identity: true, groups: [] });
		}
		let pending: FieldGroup[] = [];
		for (const group of groups) {
			const last = cards.at(-1);
			if (group.fields.length === 1) {
				if (last) last.groups.push(group);
				else pending.push(group);
				continue;
			}
			cards.push({
				key: group.name ?? '_ungrouped',
				title: groupTitle(group.name),
				identity: false,
				groups: [...pending, group]
			});
			pending = [];
		}
		if (pending.length > 0) {
			cards.push({
				key: pending[0].name ?? '_ungrouped',
				title: groupTitle(pending[0].name),
				identity: false,
				groups: pending
			});
		}
		return cards;
	}

	// Standard mode: the identity fields and each field group. Compact mode keeps the name on the
	// surface above the groups.
	function cardsFor(groups: FieldGroup[]): FieldCard[] {
		return compact
			? buildFieldCards(false, groups)
			: buildFieldCards(section !== 'fields', section !== 'identity' ? groups : []);
	}

	// One card's worth of fields sits on the modal surface; two or more cards each get a title.
	// Decided from every field of the type, so a field-set choice that hides a group (SNMPv3
	// authNoPriv hides Privacy) shows or hides fields without switching the layout to flat.
	let useGroupCards = $derived(cardsFor(groupFieldsByGroup(typeFields)).length > 1);

	// Fields are all carded or all flat: once groups are carded, compact mode's name gets a
	// Details card too, as standard mode's identity fields do.
	let fieldCards = $derived(
		compact && useGroupCards
			? buildFieldCards(true, visibleFieldGroups)
			: cardsFor(visibleFieldGroups)
	);

	// Track target IPs as local $state for reactivity (TanStack Form doesn't drive Svelte 5 reactivity)
	let targetIpValues = $state<string[]>(['']);
	let hasDaemonHostTarget = $derived(targetIpValues.some(isDaemonHostValue));
	// Wizard cards that show both targets and fields number them: pick a target, then configure.
	let showSteps = $derived(compact && !hideTargets && !hideFields);
	// Until a target row exists, the add-target buttons are the card's primary action.
	let targetCtaClass = $derived(
		`${targetIpValues.length > 0 ? 'btn-secondary' : 'btn-primary'} px-3 py-1.5 text-sm`
	);

	// --- Secret/file field mode tracking ---
	let secretFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let fileFieldModes = $state<Record<string, 'inline' | 'filepath'>>({});
	let secretFieldVisible = $state<Record<string, boolean>>({});

	// The SSH login a type that runs over one names. In `$state` for the same reason as
	// `daemonOs`; callers assembling the Credential read it through `getLoginCredentialId()`.
	let loginCredentialId = $state<string | null>(null);
	let loginError = $state<string | null>(null);
	let needsLogin = $derived(requiresLogin(selectedTypeId));

	/** The credential's `login_credential_id`: set only for a type that runs over a login. */
	export function getLoginCredentialId(): string | null {
		return needsLogin ? loginCredentialId : null;
	}

	/** Whether the login the type needs is chosen, marking the picker when it is not. */
	export function validateLogin(): boolean {
		loginError = needsLogin && !loginCredentialId ? credentials_sshLoginRequired() : null;
		return loginError === null;
	}

	// The OS of the daemons that read this credential's files and sockets. Kept in `$state`
	// because TanStack's field state does not drive Svelte 5 reactivity; callers assembling the
	// Credential read it through `getDaemonOs()`.
	let daemonOs = $state<OsFamily>('Unix');

	/** The credential's `daemon_os`: set only when something is read on the daemon. */
	export function getDaemonOs(): OsFamily | null {
		return readsDaemonFiles ? daemonOs : null;
	}

	// A caller that fixed the daemon OS owns it: keep the placeholders and the submitted value on it.
	$effect(() => {
		if (fixedDaemonOs) daemonOs = fixedDaemonOs;
	});

	function getDefaultValues(): Credential {
		if (credential) return { ...credential };
		if (organization) return createDefaultCredential(organization.id);
		return createDefaultCredential('');
	}

	function initFieldValues(ct: CredentialType) {
		fieldValues = credentialFieldValues(ct, credentialTypes.getMetadata(ct.type)?.fields ?? []);
		syncFieldsToForm(ct.type, 'all');
	}

	function initDefaultFieldValues(typeId: string) {
		const meta = credentialTypes.getMetadata(typeId);
		const fields: FieldDefinition[] = meta?.fields ?? [];
		const values: Record<string, string> = {};
		for (const field of fields) {
			values[field.id] = defaultFieldValue(field);
		}
		fieldValues = values;
		syncFieldsToForm(typeId, 'selects');
	}

	// Fields render from `fieldValues` and only push into the TanStack form on change, so a
	// field the user never touches has no form value at all. Its submit validator still runs
	// and reads `undefined` as empty, so seeding is what lets a value already on screen count
	// as present.
	//
	// `'selects'` covers a new credential: a manually-rendered select or radio must not force the user
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
			if (which === 'selects' && field.field_type !== 'select' && field.field_type !== 'radio')
				continue;
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
		daemonOs = fixedDaemonOs ?? credential?.daemon_os ?? 'Unix';
		loginCredentialId = credential?.login_credential_id ?? null;
		loginError = null;

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
					// A script source carries its mode in its own value; see `getScriptSource`.
					if (fieldDef?.field_type === 'scriptsource') continue;
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
			const typeId = fixedCredentialType ?? typeChoices?.[0] ?? 'SnmpCommunity';
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
	 * Without this a broadcast mode chosen for a Site-capable type would survive a switch
	 * to one that excludes Site: the toggle hides, but the mode — and the scope it emits —
	 * stays broadcast, producing a target the server discards.
	 */
	function applyDefaultTargetForType(typeId: string) {
		const supported = (credentialTypes.getMetadata(typeId)?.targets ?? []) as string[];
		targetMode = supported.includes('Site') ? 'broadcast' : 'per_host';
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
		if (onSubmitRequest) {
			onSubmitRequest();
			return;
		}
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
			if (field.field_type === 'scriptsource') {
				const source = parseScriptSource(value);
				typeObj[field.id] = field.optional && !scriptSourceText(source).trim() ? null : source;
			} else if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
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
	let typeOptions = $derived(
		typeChoices?.length
			? credentialTypes.getItems().filter((t) => typeChoices.includes(t.id))
			: credentialTypes.getItems()
	);

	// A type the plan lacks stays listed behind an upgrade badge, as export formats do, and picking
	// it opens the plan picker. Only creating is gated; the type is locked while editing.
	function upgradeFeatureFor(typeId: string) {
		return isEditing ? null : credentialTypeUpgradeFeature(typeId, organization?.plan?.type);
	}

	function handleGatedTypeClick(typeId: string) {
		const feature = upgradeFeatureFor(typeId);
		if (!feature) return;
		triggerUpgrade({ feature, source: 'credential_type_picker', surface: 'credential_form' });
	}

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

	// --- Script source helpers ---
	// The mode lives inside the value ({mode, path} / {mode, value}), so unlike the
	// path-or-inline fields there is no separate mode map to keep in step.
	function getScriptSource(fieldId: string): ScriptSource {
		return parseScriptSource(fieldValues[fieldId]);
	}

	function setScriptSource(fieldId: string, source: ScriptSource) {
		fieldValues[fieldId] = JSON.stringify(source);
		onChange?.({ fieldValues: { ...fieldValues } });
	}

	function scriptSourceOptions(): { value: ScriptSourceMode; label: string }[] {
		return [
			{ value: 'HostFile', label: credentials_fileOnScannedHost() },
			{ value: 'DaemonFile', label: credentials_fileOnDaemonHost() },
			{ value: 'Inline', label: common_enterValue() }
		];
	}

	/** The field's placeholder for the values on screen now, `placeholder_by` applied. */
	function placeholderFor(field: FieldDefinition): string {
		return resolvePlaceholder(field, fieldValues, daemonOs);
	}

	/**
	 * Whether anything on screen is read off the daemon's own machine: a file-mode secret or
	 * value, a daemon-side script, or a filled-in field whose placeholder follows `daemon_os`
	 * (the container sockets). Only then does the daemon OS mean anything, so only then is it
	 * asked. Mirrors the backend's `reads_daemon_paths`.
	 */
	let readsDaemonFiles = $derived(
		currentFields
			.filter((field) => daemonOsDependents.includes(field.id))
			.some((field) => {
				switch (field.field_type) {
					case 'secretpathorinline':
						return getSecretFieldMode(field.id) === 'filepath';
					case 'pathorinline':
						return getFileFieldMode(field.id) === 'filepath';
					case 'scriptsource':
						return getScriptSource(field.id).mode === 'DaemonFile';
					// A plain field the backend lists (a socket path) reads off the daemon once filled.
					default:
						return !!fieldValues[field.id]?.trim();
				}
			})
	);

	let showDaemonOsPicker = $derived(readsDaemonFiles && !fixedDaemonOs);

	// Which fields depend on the Daemon OS, computed by the backend (`os_fields`). The picker sits
	// with its field when exactly one depends on it, and above the first when several do; placement
	// follows the fields that can depend on it, not the ones in a dependent mode now, so it does
	// not jump as modes change. The Scanned Host OS is an ordinary field and stays where declared.
	let daemonOsDependents = $derived(
		credentialTypes.getMetadata(selectedTypeId)?.os_fields?.daemon ?? []
	);

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
	// Hosts -> Sites can't block submission). Reads the live, reactive `targetMode`.
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
			// A field its field set hides is not part of the credential, whatever it last held.
			if (!currentFields.some((shown) => shown.id === field.id)) return undefined;
			// For path-or-inline fields, check the actual display value, not the JSON wrapper
			let effectiveValue = value;
			if (field.field_type === 'scriptsource') {
				effectiveValue = scriptSourceText(getScriptSource(field.id));
			} else if (field.field_type === 'secretpathorinline' || field.field_type === 'pathorinline') {
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
			if (field.inline_format === 'proxmoxtokenid') {
				return proxmoxTokenId(effectiveValue);
			}
			return undefined;
		};
		return { onBlur: validate, onSubmit: validate };
	}
</script>

<!-- The selected type's integration guide. One rendering for the dedicated modal (top of the
     identity section, with the other feedback boxes) and the daemon/discovery wizards (top of each
     compact row). -->
{#snippet integrationDocsHint()}
	{#if integrationDocsPath}
		<DocsHint
			text={credentials_docsIntegration()}
			href={docsUrl(integrationDocsPath)}
			linkText={credentials_docsIntegrationLinkText({ integration: integrationName })}
		/>
	{/if}
{/snippet}

{#snippet stepHeading(number: number, label: string, help?: string)}
	{#if showSteps}
		<h3
			class="flex items-center gap-1 border-b border-(--color-border) pb-2 text-sm font-semibold text-primary"
		>
			{common_stepNumber({ number: String(number) })}
			{label}
			{#if help}
				<span
					use:tooltip
					data-tooltip={help}
					aria-label={help}
					class="inline-flex shrink-0 items-center p-0.5 text-tertiary"
					><HelpCircle class="h-3.5 w-3.5" /></span
				>
			{/if}
		</h3>
	{/if}
{/snippet}

{#if compact}
	<div class="space-y-4">
		{@render integrationDocsHint()}

		{#if !hideTargets}
			{@render stepHeading(
				1,
				daemons_credentialWizardStepSelectTarget(),
				daemons_credentialWizardTargetIpHelp()
			)}
			<!-- Hosts this credential already reaches through the host/credential junction.
			     Informational, like the site-wide credential line in the wizard — these are
			     owned elsewhere, so they are listed rather than offered as editable rows.
			     Shown in both modes: it is a fact about the credential, not about the choice
			     being made here. -->
			{#if lockedHosts.length > 0}
				<p class="flex flex-wrap items-center gap-1 text-xs text-tertiary">
					<span>{credentials_alreadyAssignedTo()}</span>
					{#each lockedHosts as host (host.id)}
						<EntityTag
							entityRef={entityRef('Host', host.id, host)}
							label={hostDisplayName(host)}
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
				<p class="text-xs text-muted">{daemons_credentialWizardBroadcastHelp()}</p>
			{:else}
				{#each targetIpValues as ip, i (i)}
					{@const removeBlocked = isDaemonHostValue(ip) && !supportsRemoteHosts}
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
						<!-- A type that can only target the daemon host keeps that row: removing it would
						     leave the credential nothing to run on. -->
						<span
							class="flex shrink-0"
							data-tooltip={removeBlocked
								? daemons_credentialWizardDaemonHostTargetFixed()
								: undefined}
							use:tooltip
						>
							<button
								type="button"
								class="p-1 text-lg leading-none text-muted hover:text-primary disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:text-muted"
								disabled={removeBlocked}
								onclick={() => handleRemoveTarget(i)}>&times;</button
							>
						</span>
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
								<button type="button" class={targetCtaClass} disabled>
									<Plus size={14} />
									{daemons_credentialWizardAddDaemonHostTarget()}
								</button>
							</span>
						{:else if hasDaemonHostTarget}
							<!-- Already added: disabled -->
							<button type="button" class={targetCtaClass} disabled>
								<Plus size={14} />
								{daemons_credentialWizardAddDaemonHostTarget()}
							</button>
						{:else}
							<button type="button" class={targetCtaClass} onclick={handleAddDaemonHostTarget}>
								<Plus size={14} />
								{daemons_credentialWizardAddDaemonHostTarget()}
							</button>
						{/if}
					{/if}
					{#if supportsRemoteHosts}
						<button type="button" class={targetCtaClass} onclick={handleAddIpTarget}>
							<Plus size={14} />
							{daemons_credentialWizardAddRemoteHostTarget()}
						</button>
					{/if}
				</div>
			{/if}
		{/if}

		<!-- Name + fields, or `fieldsNotice` in their place. -->
		{#if !hideFields}
			{@render stepHeading(2, daemons_credentialWizardStepConfigure())}
			{#if fieldsNotice}
				<InlineInfo body={fieldsNotice} />
			{:else}
				<!-- With carded groups the name sits in the Details card instead. -->
				{#if !useGroupCards}
					{@render compactNameField()}
				{/if}
				{#if needsLogin}
					{@render loginField()}
				{/if}
				{@render fieldCardList()}
			{/if}
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
		<!-- Standard mode: name/type and the field groups, laid out by `fieldCards`. A caller showing
		     one section unmounts the other's fields, so validation only reaches what is on screen; the
		     values and modes live in this component and the form, and survive. -->
		{#if section !== 'fields'}
			{@render integrationDocsHint()}
		{/if}

		{#if needsLogin && section !== 'identity'}
			{@render loginField()}
		{/if}

		{@render fieldCardList()}
	</form>
{/if}

{#snippet fieldCardList()}
	{#each fieldCards as card (card.key)}
		{#if useGroupCards}
			<InfoCard title={card.title}>
				{@render fieldCardContents(card)}
			</InfoCard>
		{:else}
			{@render fieldCardContents(card)}
		{/if}
	{/each}
{/snippet}

{#snippet fieldCardContents(card: FieldCard)}
	{#if card.identity}
		<div class="space-y-4">
			{#if compact}
				{@render compactNameField()}
			{:else}
				{@render identityFields()}
			{/if}
		</div>
	{/if}
	{#each card.groups as group (group.name ?? '_ungrouped')}
		{@render fieldList(group.fields)}
	{/each}
{/snippet}

{#snippet identityFields()}
	{#if showName}
		{@render nameField()}
		{@render descriptionField()}
	{/if}
	{#if showTypeSelector}
		{@render typeField()}
	{/if}
{/snippet}

{#snippet compactNameField()}
	<form.Field
		name={nameFieldName}
		validators={{
			onBlur: ({ value }: { value: string }) => required(value) || max(100)(value),
			onSubmit: ({ value }: { value: string }) => required(value) || max(100)(value)
		}}
		listeners={{ onChange: ({ value }: { value: string }) => onChange?.({ name: value }) }}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextInput label={common_name()} id="credential-name-{fieldPrefix}" {field} required />
		{/snippet}
	</form.Field>
{/snippet}

{#snippet nameField()}
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
{/snippet}

{#snippet descriptionField()}
	<form.Field
		name="description"
		validators={{
			onBlur: ({ value }: { value: string | null }) => max(500)(value || '')
		}}
	>
		{#snippet children(field: AnyFieldApi)}
			<TextArea
				label={common_description()}
				id="credential-description"
				{field}
				placeholder={credentials_descriptionPlaceholder()}
			/>
		{/snippet}
	</form.Field>
{/snippet}

{#snippet typeField()}
	<div class="space-y-2">
		<RichSelect
			label={credentials_credentialType()}
			selectedValue={selectedTypeId}
			options={typeOptions}
			displayComponent={CredentialTypeDisplay}
			showSearch={true}
			disabled={isEditing}
			onSelect={handleTypeChange}
			getOptionContext={(option) => ({
				upgradeFeature: upgradeFeatureFor(option.id),
				disabledReason: isEditing ? null : (typeDisabledReason?.(option.id) ?? null)
			})}
			onDisabledClick={handleGatedTypeClick}
		/>
		{#if !isEditing}
			<p class="mt-1 text-xs text-muted">{credentials_typeImmutableWarning()}</p>
		{/if}
	</div>
{/snippet}

{#snippet loginField()}
	<!-- `loginCredentialId` is the value shown, since TanStack's field state does not drive Svelte 5
	     reactivity; the field carries it into the form so its submit validation covers it. -->
	<form.Field
		name="{fieldPrefix}login_credential_id"
		validators={{
			onSubmit: () => (validateLogin() ? undefined : credentials_sshLoginRequired())
		}}
	>
		{#snippet children(formField: AnyFieldApi)}
			<LoginPicker
				selectedId={loginCredentialId}
				onSelect={(id) => {
					loginCredentialId = id;
					loginError = null;
					formField.handleChange(id);
				}}
				error={loginError}
			/>
		{/snippet}
	</form.Field>
{/snippet}

{#snippet daemonOsPicker()}
	<form.Field name="{fieldPrefix}{DAEMON_OS_FIELD}">
		{#snippet children(formField: AnyFieldApi)}
			<div class="space-y-1">
				<RadioGroup
					label={credentials_daemonOs()}
					id="{fieldPrefix}{DAEMON_OS_FIELD}"
					field={formField}
					options={osFamilyOptions()}
					value={daemonOs}
					onChange={(os) => (daemonOs = os)}
					variant="inline"
				/>
				<p class="text-xs text-muted">{credentials_daemonOsHelp()}</p>
			</div>
		{/snippet}
	</form.Field>
{/snippet}

{#snippet fieldRenderer(field: FieldDefinition, isSecret: boolean)}
	{@const fName = fieldName(field.id)}
	<div class="space-y-1">
		{#if field.field_type === 'select'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="block text-sm font-medium text-secondary">
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
						class="select-trigger w-full rounded-md px-3 py-2 text-sm text-primary"
						class:select-trigger-error={formField.state.meta.errors?.length > 0}
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
					<label for={field.id} class="block text-sm font-medium text-secondary">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<div class="space-y-2">
						<SegmentedControl
							options={[
								{ value: 'inline', label: common_enterValue() },
								{ value: 'filepath', label: credentials_fileOnDaemonHost() }
							]}
							selected={getSecretFieldMode(field.id)}
							onchange={(v) => setSecretFieldMode(field.id, v as 'inline' | 'filepath')}
							size="sm"
						/>
						{#if getSecretFieldMode(field.id) === 'inline'}
							<p class="text-xs text-muted">
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
										placeholder={placeholderFor(field) || '-----BEGIN PRIVATE KEY-----'}
										rows={4}
										class="input-field w-full rounded-md px-3 py-2 pr-10 font-mono text-sm text-primary"
										class:password-field={!secretFieldVisible[field.id]}
										class:input-field-error={formField.state.meta.errors?.length > 0}
									></textarea>
									{#if getSecretFieldDisplayValue(field.id) && getSecretFieldDisplayValue(field.id) !== '********'}
										<button
											type="button"
											class="absolute top-2 right-2 text-muted hover:text-secondary"
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
										placeholder={placeholderFor(field)}
										class="input-field w-full rounded-md px-3 py-2 pr-10 text-sm text-primary"
										class:input-field-error={formField.state.meta.errors?.length > 0}
									/>
									{#if getSecretFieldDisplayValue(field.id) && getSecretFieldDisplayValue(field.id) !== '********'}
										<button
											type="button"
											class="absolute top-1/2 right-2 -translate-y-1/2 text-muted hover:text-secondary"
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
							<p class="text-xs text-muted">
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
								placeholder={filePathPlaceholder(field, daemonOs)}
								class="input-field w-full rounded-md px-3 py-2 text-sm text-primary"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'pathorinline'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="block text-sm font-medium text-secondary">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<div class="space-y-2">
						<SegmentedControl
							options={[
								{ value: 'inline', label: common_enterValue() },
								{ value: 'filepath', label: credentials_fileOnDaemonHost() }
							]}
							selected={getFileFieldMode(field.id)}
							onchange={(v) => setFileFieldMode(field.id, v as 'inline' | 'filepath')}
							size="sm"
						/>
						{#if getFileFieldMode(field.id) === 'inline'}
							<p class="text-xs text-muted">
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
									placeholder={placeholderFor(field)}
									rows={4}
									class="input-field w-full rounded-md px-3 py-2 font-mono text-sm text-primary"
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
									placeholder={placeholderFor(field)}
									class="input-field w-full rounded-md px-3 py-2 text-sm text-primary"
									class:input-field-error={formField.state.meta.errors?.length > 0}
								/>
							{/if}
						{:else}
							<p class="text-xs text-muted">
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
								placeholder={filePathPlaceholder(field, daemonOs)}
								class="input-field w-full rounded-md px-3 py-2 text-sm text-primary"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'radio'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<RadioGroup
						label={field.label}
						id="{fieldPrefix}{field.id}"
						field={formField}
						options={field.options ?? []}
						required={!field.optional}
						value={fieldValues[field.id] ?? field.default_value ?? ''}
						onChange={(v) => handleFieldValueChange(field.id, v)}
						variant="inline"
					/>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'scriptsource'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					{@const source = getScriptSource(field.id)}
					<label for={field.id} class="block text-sm font-medium text-secondary">
						{field.label}
						{#if !field.optional}
							<span class="text-red-400">*</span>
						{/if}
					</label>
					<div class="space-y-2">
						<SegmentedControl
							options={scriptSourceOptions()}
							selected={source.mode}
							onchange={(v) =>
								setScriptSource(field.id, withScriptSourceMode(source, v as ScriptSourceMode))}
							size="sm"
						/>
						{#if source.mode === 'Inline'}
							<p class="text-xs text-muted">
								{credentials_secretStoredInDatabase()}
							</p>
							<textarea
								id={field.id}
								value={source.value}
								oninput={(e) => {
									const target = e.target as HTMLTextAreaElement;
									setScriptSource(field.id, withScriptSourceText(source, target.value));
									formField.handleChange(target.value);
								}}
								onblur={() => formField.handleBlur()}
								rows={8}
								class="input-field w-full rounded-md px-3 py-2 font-mono text-sm text-primary"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							></textarea>
						{:else}
							<p class="text-xs text-muted">
								{source.mode === 'DaemonFile'
									? credentials_filePathReadByDaemon()
									: credentials_filePathRunByScannedHost()}
							</p>
							<input
								id={field.id}
								type="text"
								value={source.path}
								oninput={(e) => {
									const target = e.target as HTMLInputElement;
									setScriptSource(field.id, withScriptSourceText(source, target.value));
									formField.handleChange(target.value);
								}}
								onblur={() => formField.handleBlur()}
								placeholder={source.mode === 'HostFile'
									? placeholderFor(field)
									: filePathPlaceholder(field, daemonOs)}
								class="input-field w-full rounded-md px-3 py-2 text-sm text-primary"
								class:input-field-error={formField.state.meta.errors?.length > 0}
							/>
						{/if}
					</div>
				{/snippet}
			</form.Field>
		{:else if field.field_type === 'text'}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="block text-sm font-medium text-secondary">
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
						placeholder={placeholderFor(field)}
						rows={8}
						class="input-field w-full rounded-md px-3 py-2 font-mono text-sm text-primary"
						class:password-field={isSecret}
						class:input-field-error={formField.state.meta.errors?.length > 0}
					></textarea>
				{/snippet}
			</form.Field>
		{:else}
			<form.Field name={fName} validators={getFieldValidators(field)}>
				{#snippet children(formField: AnyFieldApi)}
					<label for={field.id} class="block text-sm font-medium text-secondary">
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
						placeholder={placeholderFor(field)}
						class="input-field w-full rounded-md px-3 py-2 text-sm text-primary"
						class:input-field-error={formField.state.meta.errors?.length > 0}
					/>
				{/snippet}
			</form.Field>
		{/if}

		{#if field.help_text}
			<p class="text-xs text-muted">{field.help_text}</p>
		{/if}
		{#if field.field_type === 'scriptsource' && integrationDocsPath}
			<DocsHint
				text={credentials_scriptOutputDocs()}
				href={docsUrl(`${integrationDocsPath}#what-gets-discovered`)}
				linkText={credentials_scriptOutputDocsLinkText()}
			/>
		{/if}

		<!-- An OS choice only this field depends on sits with it, under its value/path toggle. -->
		{#if showDaemonOsPicker && daemonOsDependents.length === 1 && daemonOsDependents[0] === field.id}
			<div class="pt-2">{@render daemonOsPicker()}</div>
		{/if}
	</div>
{/snippet}

{#snippet fieldList(fields: FieldDefinition[])}
	{#each fieldRows(fields) as row (row[0].id)}
		<!-- An OS choice several fields depend on comes once, above the first of them. -->
		{#if showDaemonOsPicker && daemonOsDependents.length > 1 && row.some((field) => daemonOsDependents[0] === field.id)}
			{@render daemonOsPicker()}
		{/if}
		{#if row.length === 2}
			<div class="grid grid-cols-1 items-start gap-4 sm:grid-cols-2">
				{#each row as field (field.id)}
					{@render fieldRenderer(field, field.secret)}
				{/each}
			</div>
		{:else}
			{@render fieldRenderer(row[0], row[0].secret)}
		{/if}
	{/each}
{/snippet}

<style>
	.password-field {
		-webkit-text-security: disc;
	}
</style>
