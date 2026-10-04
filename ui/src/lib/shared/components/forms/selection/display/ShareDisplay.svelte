<script lang="ts" module>
	import { entities } from '$lib/shared/stores/metadata';
	import {
		common_disabled,
		common_expiresOn,
		common_untitled,
		shares_neverExpires
	} from '$lib/paraglide/messages';
	import { formatDate } from '$lib/shared/utils/formatting';

	// eslint-disable-next-line @typescript-eslint/no-empty-object-type
	export interface ShareDisplayContext {}

	export const ShareDisplay: EntityDisplayComponent<Share, ShareDisplayContext> = {
		getId: (share: Share) => share.id,
		getLabel: (share: Share) => share.name || common_untitled(),
		// Status shows as the Disabled tag, so the description carries only the expiry.
		getDescription: (share: Share) =>
			share.expires_at
				? common_expiresOn({ date: formatDate(share.expires_at) })
				: shares_neverExpires(),
		getIcon: () => entities.getIconComponent('Share'),
		getIconColor: () => entities.getColorHelper('Share').icon,
		getTags: (share: Share) => {
			const tags: TagProps[] = [];
			if (!share.is_enabled) {
				tags.push({ label: common_disabled(), color: 'Red' });
			}
			return tags;
		},
		getCategory: () => null
	};
</script>

<script lang="ts">
	import ListSelectItem from '$lib/shared/components/forms/selection/ListSelectItem.svelte';
	import type { EntityDisplayComponent } from '../types';
	import type { Share } from '$lib/features/shares/types/base';
	import type { TagProps } from '$lib/shared/components/data/types';

	interface Props {
		item: Share;
		context: ShareDisplayContext;
	}

	let { item, context }: Props = $props();
</script>

<ListSelectItem {item} {context} displayComponent={ShareDisplay} />
