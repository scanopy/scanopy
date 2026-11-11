/**
 * TanStack Form utilities for Svelte
 */
import { createFormCreator, createFormCreatorContexts } from '@tanstack/svelte-form';
import type { AnyFormApi } from '@tanstack/form-core';
import { pushError } from '#lib/shared/stores/feedback.ts';
import { common_validation_fixFields } from '#lib/paraglide/messages.js';

// Create context accessors for child components
export const { useFieldContext, useFormContext } = createFormCreatorContexts();

// Create the form factory (for context-based field access if needed)
export const { createAppForm } = createFormCreator({
	fieldComponents: {},
	formComponents: {}
});

/**
 * Check if a field is currently mounted (has an active Field component instance).
 * Stale fields from previous form sessions may linger in fieldMeta due to a
 * TanStack Form bug where form.reset() reuses the same defaultFieldMeta constant,
 * causing fieldMetaDerived's reference equality check to skip error recomputation.
 */
function isFieldMounted(form: AnyFormApi, fieldName: string): boolean {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const info = (form as any).fieldInfo?.[fieldName];
	return !info || !!info.instance;
}

/**
 * Validate a form and show user-friendly error feedback.
 * Returns true if form is valid, false otherwise.
 *
 * @param form - The TanStack Form instance
 * @param visibleFields - Optional set of field names to scope validation to
 * @param entityNameResolver - Optional function to map field paths to human-readable names
 *
 * Usage:
 * ```svelte
 * const isValid = await validateForm(form);
 * if (isValid) { nextStep(); }
 * ```
 */
export async function validateForm(
	form: AnyFormApi,
	visibleFields?: Set<string>,
	entityNameResolver?: (fieldPath: string) => string
): Promise<boolean> {
	// Validate all fields first
	await form.validateAllFields('submit');

	// Check for validation errors, filtering out:
	// - Hidden fields (if visibleFields provided)
	// - Stale unmounted fields from previous form sessions
	const errorFields = Object.entries(form.state.fieldMeta)
		.filter(([name, meta]) => {
			if (!isFieldMounted(form, name)) return false;
			if (visibleFields && !visibleFields.has(name)) return false;
			return meta?.errors && meta.errors.length > 0;
		})
		.map(([name]) => name);

	if (errorFields.length > 0) {
		// Dedupe: a resolver can map several field paths to one label (e.g. multiple
		// fields of the same credential → its name).
		const fieldNames = [
			...new Set(
				errorFields.map((f) => (entityNameResolver ? entityNameResolver(f) : f.replace(/_/g, ' ')))
			)
		].join(', ');
		pushError(common_validation_fixFields({ fields: fieldNames }));
		return false;
	}

	return true;
}

/**
 * Clean up stale field registrations from a form instance.
 * Call after form.reset() to prevent stale fieldMeta from accumulating
 * across form reuse sessions (e.g., modal open/close cycles).
 */
export function clearStaleFieldInfo(form: AnyFormApi): void {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const fieldInfo = (form as any).fieldInfo;
	if (!fieldInfo) return;
	for (const key of Object.keys(fieldInfo)) {
		if (!fieldInfo[key].instance) {
			delete fieldInfo[key];
		}
	}
}

/**
 * Submit a form with user-friendly validation feedback.
 * Shows a pushError notification if there are validation errors.
 *
 * Usage:
 * ```svelte
 * <form onsubmit={(e) => { e.preventDefault(); e.stopPropagation(); submitForm(form); }}>
 * ```
 */
export async function submitForm(
	form: AnyFormApi,
	entityNameResolver?: (fieldPath: string) => string
): Promise<void> {
	const isValid = await validateForm(form, undefined, entityNameResolver);
	if (!isValid) {
		return;
	}

	// Submit the form
	await form.handleSubmit();
}
