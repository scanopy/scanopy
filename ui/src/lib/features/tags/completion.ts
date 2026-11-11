import type { Tag } from '#lib/features/tags/types/base.ts';

/**
 * The tag the typed text completes to, or `null`. The text, trimmed and ignoring case, must start
 * a tag's name; a tag already chosen is not offered again. Of several, an exact name wins, then the
 * shortest name (the fewest letters left to type), then the alphabetically first.
 *
 * Shown as a ghost tag beside the input (`TagCompletionGhost`); Tab accepts it.
 */
export function tagCompletion(text: string, tags: Tag[], chosenIds: string[]): Tag | null {
	const typed = text.trim().toLowerCase();
	if (!typed) return null;
	const candidates = tags.filter(
		(tag) => !chosenIds.includes(tag.id) && tag.name.toLowerCase().startsWith(typed)
	);
	candidates.sort(
		(a, b) =>
			Number(b.name.toLowerCase() === typed) - Number(a.name.toLowerCase() === typed) ||
			a.name.length - b.name.length ||
			a.name.localeCompare(b.name)
	);
	return candidates[0] ?? null;
}
