import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Guards against hard-coded English UI strings that bypass paraglide i18n.
 *
 * Two conservative classes are enforced (element text is deliberately NOT scanned —
 * it is too noisy for CI):
 *   1. Literal values in label-ish component attributes.
 *   2. Literal first arguments to the push*() toast helpers.
 *
 * Legitimate literals (brand names, protocol tokens, etc.) are listed in the
 * allowlists below. Prefer fixing the string over adding an allowlist entry.
 */

// Attributes whose values are user-visible copy and must come from paraglide.
const SCANNED_ATTRIBUTES = [
	'label',
	'placeholder',
	'title',
	'aria-label',
	'subtitle',
	'emptyMessage',
	'data-tooltip'
];

// Exact attribute values that are allowed to be literal (brand names, formats,
// technical tokens). Matched case-sensitively against the full attribute value.
const ALLOWED_ATTRIBUTE_VALUES = new Set<string>([
	'JSON',
	'Scanopy',
	'GitHub',
	'Docker',
	'Confluence Wiki',
	'HTML Page',
	'PDF Document',
	'PNG Image',
	'SVG Image',
	'Mermaid (.mmd)'
]);

// Toast helper calls whose first argument must not be a string literal.
const TOAST_HELPERS = ['pushError', 'pushSuccess', 'pushWarning', 'pushInfo', 'pushToast'];

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

/**
 * A value "looks like English copy" when it is a multi-word phrase or a
 * capitalized word (Uppercase-then-lowercase). Interpolated values (containing
 * `{`) and non-alphabetic values are skipped to stay conservative.
 */
function looksLikeCopy(value: string): boolean {
	if (value.length === 0) return false;
	if (value.includes('{')) return false; // dynamic / interpolated
	if (!/[a-zA-Z]/.test(value)) return false;
	const hasSpace = value.includes(' ');
	const isCapitalizedWord = /^[A-Z][a-z]/.test(value);
	return hasSpace || isCapitalizedWord;
}

describe('i18n hard-coded strings', () => {
	const srcPath = path.resolve(__dirname, '..');
	const svelteFiles = findFilesRecursively(srcPath, ['.svelte']);
	const codeFiles = findFilesRecursively(srcPath, ['.svelte', '.ts']);

	it('should not have hard-coded English strings in label-ish attributes', () => {
		const attrPattern = new RegExp(`(${SCANNED_ATTRIBUTES.join('|')})="([^"]*)"`, 'g');
		const violations: string[] = [];

		for (const file of svelteFiles) {
			const content = fs.readFileSync(file, 'utf8');
			const lines = content.split('\n');
			lines.forEach((line, idx) => {
				let match: RegExpExecArray | null;
				attrPattern.lastIndex = 0;
				while ((match = attrPattern.exec(line)) !== null) {
					const [, attr, value] = match;
					if (ALLOWED_ATTRIBUTE_VALUES.has(value)) continue;
					if (!looksLikeCopy(value)) continue;
					const rel = path.relative(srcPath, file);
					violations.push(`  ${rel}:${idx + 1}  ${attr}="${value}"`);
				}
			});
		}

		if (violations.length > 0) {
			expect.fail(
				`Found ${violations.length} hard-coded string(s) in label-ish attributes:\n\n${violations.join('\n')}\n\n` +
					`Replace with a paraglide message (import from '#lib/paraglide/messages.js'), ` +
					`or add the exact value to ALLOWED_ATTRIBUTE_VALUES if it is a brand/technical token.`
			);
		}
	});

	it('should not pass literal strings to push*() toast helpers', () => {
		// Matches pushError('...'), pushSuccess("..."), pushWarning(`...`), etc.
		const toastPattern = new RegExp(`\\b(${TOAST_HELPERS.join('|')})\\(\\s*['"\`]`, 'g');
		const violations: string[] = [];

		for (const file of codeFiles) {
			const content = fs.readFileSync(file, 'utf8');
			const lines = content.split('\n');
			lines.forEach((line, idx) => {
				toastPattern.lastIndex = 0;
				if (toastPattern.test(line)) {
					const rel = path.relative(srcPath, file);
					violations.push(`  ${rel}:${idx + 1}  ${line.trim()}`);
				}
			});
		}

		if (violations.length > 0) {
			expect.fail(
				`Found ${violations.length} toast call(s) with a literal string argument:\n\n${violations.join('\n')}\n\n` +
					`Pass a paraglide message instead (import from '#lib/paraglide/messages.js').`
			);
		}
	});
});
