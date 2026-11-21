import { max } from '#lib/shared/components/forms/validators.ts';

/**
 * The "Something else" answer on the signup steps: optional, and at most 200 characters once
 * trimmed. The server applies the same limit (`other_answer` in `organizations/impl/intent.rs`).
 */
export function otherAnswer({ value }: { value: string }): string | undefined {
	return max(200)(value.trim());
}
