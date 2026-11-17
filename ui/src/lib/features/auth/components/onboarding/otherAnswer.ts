import { max, min, required } from '#lib/shared/components/forms/validators.ts';

/**
 * The "Something else" answer on the signup steps: required, and 3 to 200 characters once
 * trimmed. The server applies the same bounds (`other_answer` in `organizations/impl/intent.rs`).
 */
export function otherAnswer({ value }: { value: string }): string | undefined {
	const trimmed = value.trim();
	return required(trimmed) || min(3)(trimmed) || max(200)(trimmed);
}
