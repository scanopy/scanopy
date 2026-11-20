import { describe, it, expect } from 'vitest';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Translations carry no links. A link lives in the markup: a `DocsHint` for a docs link, or a
 * `slot(key)` parameter that `MessageWithLinks` renders. There, `docs-links.test.ts` finds every
 * docs URL in the code and checks it resolves, and a translator can't drop or break the anchor.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const MESSAGES_FILE = path.resolve(__dirname, '../../../messages/en.json');

describe('i18n strings', () => {
	it('carry no inline links', () => {
		const messages = JSON.parse(fs.readFileSync(MESSAGES_FILE, 'utf8')) as Record<string, unknown>;
		const withLinks = Object.entries(messages)
			.filter(([key]) => !key.startsWith('$'))
			.filter(([, value]) => typeof value === 'string' && /<a[\s>]|href\s*=/i.test(value))
			.map(([key]) => key);

		expect(withLinks).toEqual([]);
	});
});
