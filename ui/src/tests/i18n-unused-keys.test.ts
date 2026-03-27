import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Prefixes for keys that are accessed dynamically and should be skipped
const DYNAMIC_KEY_PREFIXES = [
	'errors_' // Accessed via `errors_${error.code}` in src/lib/i18n/errors.ts
];

// Keys that are allowed to have duplicate values (intentionally same or context-specific)
const ALLOWED_DUPLICATE_KEYS = new Set([
	// errors_ keys are accessed dynamically, so duplicates with other keys are acceptable
	'errors_auth_password_invalid',
	// SNMP status labels - "Testing" appears in both admin and oper status contexts
	// These may need different translations in some languages
	'snmp_adminStatusTesting',
	'snmp_operStatusTesting',
	// System Description - used in both LLDP neighbor info and SNMP system info contexts
	// These may need different translations in some languages
	'hosts_ifEntries_lldpSysDescr',
	'hosts_snmp_sysDescr',
	// Chassis ID - used in both ifEntry LLDP info and SNMP system info contexts
	// These may need different translations in some languages
	'hosts_ifEntries_chassisId',
	'hosts_snmp_chassisId'
]);

// Keys that are allowed to be single-word without common_ prefix (context-specific technical terms)
const ALLOWED_SINGLE_WORD_KEYS = new Set([
	// SNMP protocol versions - technical terms specific to SNMP
	'snmp_versionV2c',
	'snmp_versionV2cShort',
	// SNMP status labels - context-specific status values that may need different translations than generic terms
	'snmp_adminStatusTesting',
	'snmp_operStatusUp',
	'snmp_operStatusDown',
	'snmp_operStatusTesting',
	'snmp_operStatusDormant',
	// SNMP placeholders - context-specific defaults
	'snmp_communityStringPlaceholder',
	// LLDP/CDP neighbor context - specific to network discovery protocol terminology
	'hosts_ifEntries_neighbor',
	// Discovery legacy label - context-specific label for non-Unified discovery types
	'discovery_legacyType'
]);

function findFilesRecursively(dir: string, extensions: string[]): string[] {
	const files: string[] = [];
	const entries = fs.readdirSync(dir, { withFileTypes: true });

	for (const entry of entries) {
		const fullPath = path.join(dir, entry.name);
		if (entry.isDirectory() && entry.name !== 'node_modules' && entry.name !== 'tests') {
			files.push(...findFilesRecursively(fullPath, extensions));
		} else if (entry.isFile() && extensions.some((ext) => entry.name.endsWith(ext))) {
			files.push(fullPath);
		}
	}

	return files;
}

function isDynamicKey(key: string): boolean {
	return DYNAMIC_KEY_PREFIXES.some((prefix) => key.startsWith(prefix));
}

function isKeyUsed(key: string, sourceFiles: string[], fileContents: Map<string, string>): boolean {
	// Check for both namespace imports (m.key) and named imports (key directly)
	const namespacePattern = `m.${key}(`;
	const namedPattern = `${key}(`;
	const importPattern = /from\s+['"]\$lib\/paraglide\/messages['"]/;

	for (const file of sourceFiles) {
		const content = fileContents.get(file);
		if (!content) continue;

		// Check namespace import pattern
		if (content.includes(namespacePattern)) {
			return true;
		}

		// Check named import pattern (only if file imports from paraglide)
		if (content.includes(namedPattern) && importPattern.test(content)) {
			return true;
		}
	}

	return false;
}

describe('i18n', () => {
	it('should not have unused translation keys in en.json', () => {
		const messagesPath = path.resolve(__dirname, '../../../messages/en.json');
		const srcPath = path.resolve(__dirname, '..');

		const messages = JSON.parse(fs.readFileSync(messagesPath, 'utf8'));
		const keys = Object.keys(messages).filter((key) => key !== '$schema');

		const sourceFiles = findFilesRecursively(srcPath, ['.svelte', '.ts']);

		// Pre-load all file contents for faster searching
		const fileContents = new Map<string, string>();
		for (const file of sourceFiles) {
			fileContents.set(file, fs.readFileSync(file, 'utf8'));
		}

		const unusedKeys: string[] = [];

		for (const key of keys) {
			// Skip keys that are accessed dynamically
			if (isDynamicKey(key)) {
				continue;
			}

			if (!isKeyUsed(key, sourceFiles, fileContents)) {
				unusedKeys.push(key);
			}
		}

		if (unusedKeys.length > 0) {
			const message = `Found ${unusedKeys.length} unused translation keys in en.json:\n\n${unusedKeys.map((k) => `  - ${k}`).join('\n')}\n\nRemove these keys from messages/en.json or use them in the codebase.`;
			expect.fail(message);
		}
	});

	it('should not have duplicate translation values in en.json', () => {
		const messagesPath = path.resolve(__dirname, '../../../messages/en.json');
		const messages = JSON.parse(fs.readFileSync(messagesPath, 'utf8'));

		// Group keys by their value
		const valueToKeys = new Map<string, string[]>();

		for (const [key, value] of Object.entries(messages)) {
			if (key === '$schema') continue;
			if (typeof value !== 'string') continue;

			const existing = valueToKeys.get(value) || [];
			existing.push(key);
			valueToKeys.set(value, existing);
		}

		// Find duplicates (excluding allowed duplicates)
		const duplicates: { value: string; keys: string[] }[] = [];

		for (const [value, keys] of valueToKeys) {
			if (keys.length > 1) {
				// Skip if any key in this group is in the allowed set
				const hasAllowedKey = keys.some((k) => ALLOWED_DUPLICATE_KEYS.has(k));
				if (!hasAllowedKey) {
					duplicates.push({ value, keys });
				}
			}
		}

		if (duplicates.length > 0) {
			const lines = duplicates.map(
				(d) => `  "${d.value}":\n${d.keys.map((k) => `    - ${k}`).join('\n')}`
			);
			const message = `Found ${duplicates.length} duplicate translation values in en.json:\n\n${lines.join('\n\n')}\n\nConsolidate these into common_ keys.`;
			expect.fail(message);
		}
	});

	it('should use common_ prefix for single-word translation values', () => {
		const messagesPath = path.resolve(__dirname, '../../../messages/en.json');
		const messages = JSON.parse(fs.readFileSync(messagesPath, 'utf8'));

		const violations: { key: string; value: string }[] = [];

		for (const [key, value] of Object.entries(messages)) {
			if (key === '$schema') continue;
			if (typeof value !== 'string') continue;
			if (key.startsWith('common_')) continue;
			if (isDynamicKey(key)) continue;
			if (ALLOWED_SINGLE_WORD_KEYS.has(key)) continue;

			// Single-word: no spaces and no parameter placeholders
			const isSingleWord = !value.includes(' ') && !value.includes('{');
			if (isSingleWord && value.length > 0) {
				violations.push({ key, value });
			}
		}

		if (violations.length > 0) {
			const lines = violations.map((v) => `  - ${v.key}: "${v.value}"`);
			const message = `Found ${violations.length} single-word translations without common_ prefix:\n\n${lines.join('\n')}\n\nRename these keys to use the common_ prefix (e.g., common_${violations[0]?.value.toLowerCase()}).`;
			expect.fail(message);
		}
	});
});
