#!/usr/bin/env node

/**
 * Syncs backend-metadata fixture strings into messages/en.json as meta_* keys.
 *
 * This script:
 * 1. Reads the covered fixture files from ui/src/lib/data/
 * 2. Builds meta_<fixtureKey>_<id>_name / _description keys (plus field
 *    label/placeholder/helpText keys for field-definition fixtures) with the
 *    fixture strings as English values
 * 3. Reads the existing messages/en.json
 * 4. Replaces all meta_* keys with the freshly generated set (adds missing,
 *    updates changed, removes stale)
 * 5. Preserves manual translations and other keys
 *
 * The English fixture strings remain the runtime fallback; these keys are the
 * translated path (resolved dynamically in ui/src/lib/i18n/metadata.ts).
 *
 * Run via: make generate-fixtures
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const UI_DIR = join(__dirname, '..');
const PROJECT_ROOT = join(UI_DIR, '..');

export const DATA_DIR = join(UI_DIR, 'src/lib/data');
export const MESSAGES_FILE = join(PROJECT_ROOT, 'messages/en.json');

/**
 * Covered fixture files and their fixture keys (used as the key namespace:
 * meta_<fixtureKey>_<id>_*). service-definitions/service-categories are
 * deliberately excluded (proper nouns).
 *
 * kind:
 * - 'typeMetadata': entries are {id, name, description, ...}
 * - 'fieldDefinitions': entries are flat field definitions {id, label, placeholder, help_text}
 * - 'credentialTypes': typeMetadata plus nested metadata.fields[] field definitions
 */
export const COVERED_FIXTURES = [
	{ file: 'credential-types.json', key: 'credential_types', kind: 'credentialTypes' },
	// billing-plans.json is a strict subset of billing-plans-all.json; both resolve
	// through the billing_plans namespace.
	{ file: 'billing-plans-all.json', key: 'billing_plans', kind: 'typeMetadata' },
	{ file: 'features.json', key: 'features', kind: 'typeMetadata' },
	{ file: 'discovery-types.json', key: 'discovery_types', kind: 'typeMetadata' },
	// Why a scan ended. The description is the sentence a customer reads on a failed or stalled
	// run and pastes into a support request, so it has to be translatable.
	{
		file: 'discovery-terminal-reasons.json',
		key: 'discovery_terminal_reasons',
		kind: 'typeMetadata'
	},
	{ file: 'dependency-types.json', key: 'dependency_types', kind: 'typeMetadata' },
	{ file: 'permissions.json', key: 'permissions', kind: 'typeMetadata' },
	{ file: 'scan-settings.json', key: 'scan_settings', kind: 'fieldDefinitions' },
	{ file: 'subnet-types.json', key: 'subnet_types', kind: 'typeMetadata' },
	{ file: 'container-types.json', key: 'container_types', kind: 'typeMetadata' },
	{ file: 'container-rule-types.json', key: 'container_rule_types', kind: 'typeMetadata' },
	{ file: 'element-rule-types.json', key: 'element_rule_types', kind: 'typeMetadata' },
	{ file: 'views.json', key: 'views', kind: 'typeMetadata' },
	{ file: 'ports.json', key: 'ports', kind: 'typeMetadata' },
	{ file: 'cancel-reasons.json', key: 'cancel_reasons', kind: 'typeMetadata' },
	{ file: 'save-offers.json', key: 'save_offers', kind: 'typeMetadata' },
	{ file: 'plan-statuses.json', key: 'plan_statuses', kind: 'typeMetadata' },
	// Provenance tiers. The `metadata.sources` list under each is data the UI looks values up in,
	// not copy — only the name and description are translated.
	{ file: 'attribute-methods.json', key: 'attribute_methods', kind: 'typeMetadata' },
	// Per-source labels. `Probe` and `Authored` carry a `{probe}` slot filled from the next file,
	// so their names compile to functions taking an inputs object (see `metaNameWith`).
	{ file: 'attribute-sources.json', key: 'attribute_sources', kind: 'typeMetadata' },
	{ file: 'client-probes.json', key: 'client_probes', kind: 'typeMetadata' },
	// How an entity came to exist, keyed by its `source.type`. The descriptions are operator-facing
	// explanations (the Inferred one is the notice on an inferred host), not internal notes.
	{ file: 'entity-sources.json', key: 'entity_sources', kind: 'typeMetadata' },
	// Scan warnings. Descriptions here are templates with `{named}` slots, unlike every other
	// entry above: the values are copied through verbatim, paraglide compiles them into functions
	// that take an inputs object, and `metaDescriptionWith` in src/lib/i18n/metadata.ts is what
	// calls them with one. The slot names come from the same `metadata.slots` the backend test
	// checks the template against.
	{ file: 'warning-codes.json', key: 'warning_codes', kind: 'typeMetadata' },
	// The rung each code's `category` names. Heading and sub-line for a section of the list.
	{ file: 'warning-remedies.json', key: 'warning_remedies', kind: 'typeMetadata' },
	{ file: 'snmp-walk-groups.json', key: 'snmp_walk_groups', kind: 'typeMetadata' },
	{ file: 'claim-sources.json', key: 'claim_sources', kind: 'typeMetadata' },
	// Keyed by the credential-query discriminant a warning carries, which credential-types.json
	// cannot resolve — it is keyed by CredentialType.
	{ file: 'discovery-integrations.json', key: 'discovery_integrations', kind: 'typeMetadata' },
	{
		file: 'malformed-neighbour-consequences.json',
		key: 'malformed_neighbour_consequences',
		kind: 'typeMetadata'
	}
];

/**
 * @typedef {Object} FieldDefinitionJson
 * @property {string} id
 * @property {string} [label]
 * @property {string} [placeholder]
 * @property {string} [help_text]
 * @property {{ value: string, label: string }[]} [options]
 */

/**
 * Build the meta_* keys for one field definition under the given prefix.
 * Mirrors the runtime lookup in ui/src/lib/i18n/metadata.ts.
 *
 * @param {string} prefix
 * @param {FieldDefinitionJson} field
 * @returns {Record<string, string>}
 */
function fieldMessages(prefix, field) {
	/** @type {Record<string, string>} */
	const messages = {};
	if (field.label) messages[`${prefix}_${field.id}_label`] = field.label;
	if (field.placeholder) messages[`${prefix}_${field.id}_placeholder`] = field.placeholder;
	if (field.help_text) messages[`${prefix}_${field.id}_helpText`] = field.help_text;
	for (const option of field.options ?? []) {
		if (option.label) {
			messages[`${prefix}_${field.id}_option_${option.value}`] = option.label;
		}
	}
	return messages;
}

/**
 * Build the full meta_* message map from the covered fixture files.
 *
 * @param {string} [dataDir]
 * @returns {Record<string, string>}
 */
export function buildMetaMessages(dataDir = DATA_DIR) {
	/** @type {Record<string, string>} */
	const messages = {};
	for (const { file, key, kind } of COVERED_FIXTURES) {
		const items = JSON.parse(readFileSync(join(dataDir, file), 'utf-8'));
		for (const item of items) {
			const prefix = `meta_${key}_${item.id}`;
			if (kind === 'fieldDefinitions') {
				Object.assign(messages, fieldMessages(`meta_${key}`, item));
				continue;
			}
			if (item.name) messages[`${prefix}_name`] = item.name;
			if (item.description) messages[`${prefix}_description`] = item.description;
			if (kind === 'credentialTypes') {
				for (const field of item.metadata?.fields ?? []) {
					Object.assign(messages, fieldMessages(prefix, field));
				}
			}
		}
	}
	return messages;
}

function main() {
	const metaMessages = buildMetaMessages();
	console.log(`Built ${Object.keys(metaMessages).length} meta messages from fixtures`);

	/** @type {Record<string, string>} */
	let existingMessages = {};
	if (existsSync(MESSAGES_FILE)) {
		existingMessages = JSON.parse(readFileSync(MESSAGES_FILE, 'utf-8'));
		console.log(`Read ${Object.keys(existingMessages).length} existing messages from en.json`);
	}

	// Remove old meta messages (they'll be replaced with fresh ones)
	/** @type {Record<string, string>} */
	const manualMessages = {};
	let removedCount = 0;
	for (const [key, value] of Object.entries(existingMessages)) {
		if (key.startsWith('meta_')) {
			removedCount++;
		} else {
			manualMessages[key] = value;
		}
	}
	if (removedCount > 0) {
		console.log(`Removed ${removedCount} old meta messages`);
	}

	// Merge: manual messages + new meta messages
	const merged = {
		...manualMessages,
		...metaMessages
	};

	// Sort keys for consistent output
	/** @type {Record<string, string>} */
	const sorted = {};
	for (const key of Object.keys(merged).sort()) {
		sorted[key] = merged[key];
	}

	writeFileSync(MESSAGES_FILE, JSON.stringify(sorted, null, '\t') + '\n');
	console.log(`Wrote ${Object.keys(sorted).length} total messages to en.json`);
	console.log(`  - ${Object.keys(manualMessages).length} manual messages`);
	console.log(`  - ${Object.keys(metaMessages).length} meta messages`);
}

// Only run when executed directly (the drift-guard test imports the builders)
if (process.argv[1] === fileURLToPath(import.meta.url)) {
	main();
}
