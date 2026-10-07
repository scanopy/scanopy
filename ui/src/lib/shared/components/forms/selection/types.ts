import type { TagProps } from '../../data/types';
import type { IconComponent } from '$lib/shared/utils/types';
import type { EntityDiscriminants } from '$lib/api/entities';
import type { Component } from 'svelte';

/**
 * The context fields every display reads the same way, through `displayTags`: `compact` drops the
 * display's `compactHides` roles (rows in the narrow topology inspector), `hideTags` drops the
 * named roles where the surrounding view already says what the tag would.
 */
export interface DisplayTagContext<R extends string = string> {
	compact?: boolean;
	hideTags?: R[];
}

// @typescript-eslint/no-explicit-any
export interface EntityDisplayComponent<T, C> {
	// Required methods
	getId(item: T): string;
	getLabel(item: T, context?: C): string;

	// Optional methods with defaults
	getDescription?(item: T, context: C): string;
	getIcon?(item: T, context: C): IconComponent | null;
	getIconColor?(item: T, context: C): string | null;
	getTags?(item: T, context: C): TagProps[];
	/** Tag roles a `compact` context drops. Read through `displayTags`, never directly. */
	compactHides?: readonly string[];
	getCategory?(item: T, context: C): string | null;
	getDisabled?(item: T, context: C): boolean;
	getDisabledReason?(item: T, context: C): string | null;

	// Inline tag picker support
	getTagPickerProps?(
		item: T,
		context: C
	): {
		selectedTagIds: string[];
		entityId: string;
		entityType: EntityDiscriminants;
		availableTags?: import('$lib/features/tags/types/base').Tag[];
		allowCreate?: boolean;
	} | null;

	// Inline editing support
	supportsInlineEdit?: boolean;
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	InlineEditorComponent?: Component<any>;
}
