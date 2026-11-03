<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { applicationGroupId, tagIcon, tagTooltip } from '$lib/features/tags/groups';
	import TagPickerInline from '$lib/features/tags/components/TagPickerInline.svelte';
	import { AVAILABLE_COLORS, type Color } from '$lib/shared/utils/styling';
	import { createDefaultTag } from '$lib/features/tags/types/base';
	import type { Tag as TagType } from '$lib/features/tags/types/base';
	import {
		useCreateTagMutation,
		useDeleteTagMutation,
		useTagGroupsQuery,
		useTagsQuery,
		useUpdateTagMutation
	} from '$lib/features/tags/queries';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { getSuggestions } from '../suggestions';
	import { concepts } from '$lib/shared/stores/metadata';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import DocsHint from '$lib/shared/components/feedback/DocsHint.svelte';
	import {
		appWizard_convertExisting,
		appWizard_createYourOwn,
		appWizard_defineGroupsDescription_convert,
		appWizard_defineGroupsDescription_create,
		appWizard_docsHint,
		appWizard_docsHintLinkText,
		appWizard_membershipFootnote,
		appWizard_noGroupsYet,
		appWizard_suggestedGroups,
		appWizard_mspCallout
	} from '$lib/paraglide/messages';

	let {
		appTags
	}: {
		appTags: TagType[];
	} = $props();

	const organizationQuery = useOrganizationQuery();
	const createTagMutation = useCreateTagMutation();
	const deleteTagMutation = useDeleteTagMutation();
	const updateTagMutation = useUpdateTagMutation();
	const tagsQuery = useTagsQuery();
	const tagGroupsQuery = useTagGroupsQuery();
	let tagGroups = $derived(tagGroupsQuery.data ?? []);
	let applicationGroup = $derived(applicationGroupId(tagGroups));

	let organization = $derived(organizationQuery.data);
	let useCase = $derived(organization?.use_case ?? null);
	let isMsp = $derived(useCase === 'msp');

	let suggestions = $derived(getSuggestions(useCase));
	let allTags = $derived(tagsQuery.data ?? []);
	// Only ungrouped tags can become applications: a tag in another group already means something.
	let ungroupedTags = $derived(allTags.filter((t) => !t.tag_group_id));
	// Exclude suggestions whose name collides with ANY existing tag — creating one
	// would violate the name-unique constraint, whether the existing tag is an app
	// tag or not.
	let existingNames = $derived(new Set(allTags.map((t) => t.name.toLowerCase())));
	let availableSuggestions = $derived(
		suggestions.filter((s) => !existingNames.has(s.toLowerCase()))
	);

	// Stable color per suggestion based on position in the full suggestions list
	function getSuggestionColor(name: string): Color {
		const idx = suggestions.indexOf(name);
		return AVAILABLE_COLORS[(idx >= 0 ? idx : 0) % AVAILABLE_COLORS.length];
	}

	let isCreating = $state(false);
	let isConverting = $state(false);

	async function convertToApp(tag: TagType) {
		if (isConverting) return;
		isConverting = true;
		try {
			await updateTagMutation.mutateAsync({
				...tag,
				tag_group_id: applicationGroup,
				icon: null
			});
		} finally {
			isConverting = false;
		}
	}

	function getRandomColor(): Color {
		return AVAILABLE_COLORS[Math.floor(Math.random() * AVAILABLE_COLORS.length)];
	}

	async function createAppTag(name: string, color?: Color) {
		if (!organization || isCreating) return;
		const trimmed = name.trim();
		if (!trimmed || existingNames.has(trimmed.toLowerCase())) return;

		isCreating = true;
		try {
			const tag = createDefaultTag(organization.id);
			tag.name = trimmed;
			tag.color = color ?? getRandomColor();
			tag.tag_group_id = applicationGroup;
			await createTagMutation.mutateAsync(tag);
		} finally {
			isCreating = false;
		}
	}

	const queryClient = useQueryClient();

	async function deleteAppTag(tagId: string) {
		await deleteTagMutation.mutateAsync(tagId);
		// Invalidate host/service caches so deleted tag IDs are cleaned up
		queryClient.invalidateQueries({ queryKey: queryKeys.hosts.lists() });
		queryClient.invalidateQueries({ queryKey: queryKeys.services.all });
	}
</script>

<div class="space-y-6">
	<div class="space-y-2">
		<p class="text-sm text-secondary">
			{ungroupedTags.length > 0
				? appWizard_defineGroupsDescription_convert()
				: appWizard_defineGroupsDescription_create()}
		</p>
		<DocsHint
			text={appWizard_docsHint()}
			href="https://scanopy.net/docs/guides/operations/mapping-applications/"
			linkText={appWizard_docsHintLinkText()}
		/>
	</div>

	<!-- MSP callout -->
	{#if isMsp}
		<InlineInfo title={appWizard_mspCallout()} dismissableKey="msp-client-callout" />
	{/if}

	<!-- Suggested groups as application-styled tags -->
	{#if availableSuggestions.length > 0}
		<div>
			<h3 class="mb-2 text-xs font-medium tracking-wide text-secondary uppercase">
				{appWizard_suggestedGroups()}
			</h3>
			<div class="flex flex-wrap gap-2">
				{#each availableSuggestions as suggestion (suggestion)}
					{@const suggestionColor = getSuggestionColor(suggestion)}
					<button
						type="button"
						class="cursor-pointer"
						onclick={() => createAppTag(suggestion, suggestionColor)}
						disabled={isCreating}
					>
						<Tag
							label={suggestion}
							color={suggestionColor}
							icon={concepts.getIconComponent('Application')}
							isShiny={true}
							pill={true}
						/>
					</button>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Convert existing tags -->
	{#if ungroupedTags.length > 0}
		<div>
			<h3 class="mb-2 text-xs font-medium tracking-wide text-secondary uppercase">
				{appWizard_convertExisting()}
			</h3>
			<div class="flex flex-wrap gap-2">
				{#each ungroupedTags as tag (tag.id)}
					<button
						type="button"
						class="cursor-pointer"
						onclick={() => convertToApp(tag)}
						disabled={isConverting}
					>
						<Tag
							label={tag.name}
							color={tag.color}
							icon={tagIcon(tag, applicationGroup)}
							title={tagTooltip(tag, tagGroups)}
							pill={true}
						/>
					</button>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Custom group entry via TagPickerInline -->
	<div>
		<h3 class="mb-2 text-xs font-medium tracking-wide text-secondary uppercase">
			{appWizard_createYourOwn()}
		</h3>
		<TagPickerInline
			selectedTagIds={appTags.map((t) => t.id)}
			availableTags={[...appTags, ...ungroupedTags]}
			onAdd={(tagId) => {
				const tag = ungroupedTags.find((t) => t.id === tagId);
				if (tag) convertToApp(tag);
			}}
			onRemove={(tagId) => deleteAppTag(tagId)}
			createAsApplication={true}
			onePerGroup={false}
		/>
	</div>

	<!-- Footnote / empty state -->
	<p class="text-center text-sm text-tertiary italic">
		{#if appTags.length === 0}
			{appWizard_noGroupsYet()}&nbsp;
		{/if}
		{appWizard_membershipFootnote()}
	</p>
</div>
