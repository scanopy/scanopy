import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

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

describe('i18n prop defaults', () => {
	it('should not use namespace imports for paraglide messages in Svelte files', () => {
		// Namespace imports (import * as m from '#lib/paraglide/messages.js') break in
		// production builds due to Svelte 5 + Vite bundler tree-shaking.
		//
		// The @__NO_SIDE_EFFECTS__ annotation on paraglide message functions causes the
		// bundler to aggressively tree-shake m.function() calls, even in templates,
		// resulting in "(void 0) is not a function" errors at runtime.
		//
		// BAD:  import * as m from '#lib/paraglide/messages.js';
		//       {message ?? m.common_loading()}
		//
		// GOOD: import { common_loading } from '#lib/paraglide/messages.js';
		//       {message ?? common_loading()}

		const srcPath = path.resolve(__dirname, '..');
		const svelteFiles = findFilesRecursively(srcPath, ['.svelte']);

		// Pattern: import * as <name> from '#lib/paraglide/messages.js'
		const namespaceImportPattern =
			/import\s+\*\s+as\s+(\w+)\s+from\s+['"]#lib\/paraglide\/messages\.js['"]/;

		const violations: { file: string; alias: string; line: number }[] = [];

		for (const file of svelteFiles) {
			const content = fs.readFileSync(file, 'utf8');
			const lines = content.split('\n');

			for (let i = 0; i < lines.length; i++) {
				const line = lines[i];
				const match = line.match(namespaceImportPattern);

				if (match) {
					violations.push({
						file: path.relative(srcPath, file),
						alias: match[1],
						line: i + 1
					});
				}
			}
		}

		if (violations.length > 0) {
			const lines = violations.map(
				(v) =>
					`  - ${v.file}:${v.line}\n    uses "import * as ${v.alias} from '#lib/paraglide/messages.js'"`
			);
			const message = `Found ${violations.length} Svelte files using namespace imports for paraglide messages:

${lines.join('\n')}

This pattern breaks in production builds due to tree-shaking. Instead, use named imports:
  import { common_loading, common_save } from '#lib/paraglide/messages.js';`;
			expect.fail(message);
		}
	});
});
