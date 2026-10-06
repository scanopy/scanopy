import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const SRC = path.resolve(__dirname, '..');

/**
 * No `$derived` reads TanStack form-level state.
 *
 * `form.state` is a plain getter on `FormApi`'s store, which Svelte 5's `$derived` does not
 * track. A `$derived` that reads `form.state.values` computes once and never updates: the user
 * edit modal kept the site picker hidden after permissions moved from Admin to Member, because
 * `permissionsValue` stayed at the first value it read.
 *
 * Mirror the value into `$state` from `form.store.subscribe` (see `CreateDaemonModal.svelte`)
 * and derive from that instead.
 *
 * `field.state` is exempt: `@tanstack/svelte-form`'s `Field.svelte` defines it as a getter over
 * `useStore(api.store).current`, a `$state` slice, so `$derived` tracks it.
 */

/** A read of the form's values through the store snapshot rather than a subscription. */
const FORM_STATE_READ = /\.state\.values\b/;

function svelteFiles(dir: string): string[] {
	const out: string[] = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name);
		if (entry.isDirectory()) {
			if (entry.name === 'node_modules' || entry.name === 'paraglide') continue;
			out.push(...svelteFiles(full));
		} else if (entry.name.endsWith('.svelte')) {
			out.push(full);
		}
	}
	return out;
}

/**
 * The argument of the call whose opening paren sits at `open`, matched by paren depth so a
 * multi-line `$derived.by(() => { … })` body is captured whole. Strings and comments are skipped
 * so a paren or an apostrophe inside one does not end the expression early.
 */
function callArgument(source: string, open: number): string {
	let depth = 0;
	let quote: string | null = null;
	for (let i = open; i < source.length; i++) {
		const ch = source[i];
		if (quote) {
			if (ch === '\\') i++;
			else if (ch === quote) quote = null;
			continue;
		}
		if (ch === '/' && source[i + 1] === '/') {
			const end = source.indexOf('\n', i);
			i = end === -1 ? source.length : end;
		} else if (ch === '/' && source[i + 1] === '*') {
			const end = source.indexOf('*/', i + 2);
			i = end === -1 ? source.length : end + 1;
		} else if (ch === '"' || ch === "'" || ch === '`') quote = ch;
		else if (ch === '(') depth++;
		else if (ch === ')') {
			depth--;
			if (depth === 0) return source.slice(open + 1, i);
		}
	}
	return source.slice(open + 1);
}

function stripComments(code: string): string {
	return code.replace(/\/\*[\s\S]*?\*\//g, '').replace(/\/\/[^\n]*/g, '');
}

/** Each `$derived` / `$derived.by` expression in `source` that reads form state, by line. */
function formStateDerivedReads(source: string): number[] {
	const lines: number[] = [];
	const call = /\$derived(?:\.by)?\s*\(/g;
	let match: RegExpExecArray | null;
	while ((match = call.exec(source))) {
		const open = match.index + match[0].length - 1;
		if (FORM_STATE_READ.test(stripComments(callArgument(source, open)))) {
			lines.push(source.slice(0, match.index).split('\n').length);
		}
	}
	return lines;
}

describe('form state reactivity', () => {
	it('flags single-line and multi-line reads, and ignores mirrored state and comments', () => {
		expect(formStateDerivedReads('let p = $derived(form.state.values.permissions);')).toEqual([1]);
		expect(
			formStateDerivedReads(
				"let a = 1;\nlet s = $derived.by(() => {\n\t// can't\n\tconst v = (form.state.values.name ?? '(');\n\treturn v;\n});"
			)
		).toEqual([2]);
		expect(formStateDerivedReads('let p = $derived(permissionsValue === "Member");')).toEqual([]);
		expect(
			formStateDerivedReads('let p = $derived(x); // form.state.values is not tracked')
		).toEqual([]);
	});

	it('no .svelte file reads form state inside $derived', () => {
		const violations: string[] = [];
		for (const file of svelteFiles(path.join(SRC, 'lib')).concat(
			svelteFiles(path.join(SRC, 'routes'))
		)) {
			const source = fs.readFileSync(file, 'utf8');
			for (const line of formStateDerivedReads(source)) {
				violations.push(`${path.relative(SRC, file)}:${line}`);
			}
		}
		expect(violations).toEqual([]);
	});
});
