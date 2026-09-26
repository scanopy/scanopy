<script lang="ts">
	import { required, max } from '$lib/shared/components/forms/validators';
	import type { Share } from '../types/base';
	import type { components } from '$lib/api/schema';
	type TopologyView = components['schemas']['TopologyView'];
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans } from '$lib/shared/stores/metadata';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import CodeContainer from '$lib/shared/components/data/CodeContainer.svelte';
	import CollapsibleCard from '$lib/shared/components/data/CollapsibleCard.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import { generateShareUrl, generateEmbedCode } from '../queries';
	import { Sun, Moon, Monitor } from 'lucide-svelte';
	import { views } from '$lib/shared/stores/metadata';
	import viewsJson from '$lib/data/views.json';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_enabled,
		common_height,
		common_name,
		common_password,
		common_theme,
		common_width,
		shares_accessControl,
		shares_allowedDomainsHelp,
		shares_allowedDomainsPlaceholder,
		shares_allowedEmbedDomains,
		shares_cacheInfoBody,
		shares_cacheInfoTitle,
		shares_displayOptions,
		shares_embedCode,
		shares_embedDimensions,
		shares_embedsRequirePlan,
		shares_enabledHelp,
		shares_expirationDate,
		shares_expirationHelp,
		shares_namePlaceholder,
		shares_passwordHelpEdit,
		shares_passwordPlaceholder,
		shares_shareThemeDefault,
		shares_shareThemeLight,
		shares_shareThemeDark,
		shares_shareUrl,
		shares_showExportButton,
		shares_showInspectPanel,
		shares_showZoomControls,
		shares_upgradeForEmbeds,
		shares_topologyViews,
		shares_enabledViews,
		shares_enabledViewsHelp,
		shares_allViewsEnabled,
		shares_urlAvailableAfterSaveTitle,
		shares_urlAvailableAfterSaveBody,
		topology_showMinimap
	} from '$lib/paraglide/messages';

	interface Props {
		share: Share;
		index: number;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		isSaved?: boolean;
		onChange?: (share: Share) => void;
	}

	let { share, index, form, isSaved = true, onChange = () => {} }: Props = $props();

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	let shareTheme = $state<'default' | 'light' | 'dark'>('default');

	let hasEmbedsFeature = $derived(
		organization?.plan ? billingPlans.getMetadata(organization.plan.type).features.embeds : true
	);

	// Field names for this share in the form array
	let nameFieldName = $derived(`shares[${index}].name`);
	let passwordFieldName = $derived(`shares[${index}].password`);
	let allowedDomainsFieldName = $derived(`shares[${index}].allowed_domains`);
	let expiresAtFieldName = $derived(`shares[${index}].expires_at`);
	let isEnabledFieldName = $derived(`shares[${index}].is_enabled`);
	let showZoomControlsFieldName = $derived(`shares[${index}].show_zoom_controls`);
	let showInspectPanelFieldName = $derived(`shares[${index}].show_inspect_panel`);
	let showExportButtonFieldName = $derived(`shares[${index}].show_export_button`);
	let showMinimapFieldName = $derived(`shares[${index}].show_minimap`);
	let embedWidthFieldName = $derived(`shares[${index}].embed_width`);
	let embedHeightFieldName = $derived(`shares[${index}].embed_height`);

	// Notify parent of changes for real-time sync
	function handleNameChange(value: string) {
		onChange({ ...share, name: value });
	}

	function handleEnabledChange(value: boolean) {
		onChange({ ...share, is_enabled: value });
	}

	// For embed code display
	let themeParam = $derived(shareTheme === 'default' ? undefined : shareTheme) as
		'light' | 'dark' | undefined;

	// All available views as SimpleOption items
	const allViewOptions: SimpleOption[] = viewsJson.map((v) => ({
		value: v.id,
		label: v.name,
		description: v.description,
		icon: views.getIconComponent(v.id),
		iconColor: views.getColorHelper(v.id).icon
	}));

	// Enabled views state — null means all views (empty list in the UI).
	// Seeded once from the prop; user edits override it until save.
	// svelte-ignore state_referenced_locally
	let enabledViewIds: TopologyView[] = $state(share.enabled_views ? [...share.enabled_views] : []);

	// Items currently in the list (preserves order)
	let enabledViewItems = $derived(
		enabledViewIds
			.map((id) => allViewOptions.find((v) => v.value === id))
			.filter((v): v is SimpleOption => v != null)
	);

	// Options not yet added (available for the dropdown)
	let availableViewOptions = $derived(
		allViewOptions.filter((v) => !enabledViewIds.includes(v.value as TopologyView))
	);

	function handleAddView(viewId: string) {
		enabledViewIds = [...enabledViewIds, viewId as TopologyView];
		syncEnabledViews();
	}

	function handleRemoveView(idx: number) {
		enabledViewIds = enabledViewIds.filter((_, i) => i !== idx);
		syncEnabledViews();
	}

	function syncEnabledViews() {
		// Empty list = null (all views enabled)
		const value = enabledViewIds.length === 0 ? null : enabledViewIds;
		onChange({ ...share, enabled_views: value });
	}
</script>

<div class="space-y-4 px-1">
	<InlineInfo
		title={shares_cacheInfoTitle()}
		body={shares_cacheInfoBody()}
		dismissableKey="share-cache-info"
	/>

	<!-- Name -->
	<div class="card card-static">
		<form.Field
			name={nameFieldName}
			validators={{
				onBlur: ({ value }: { value: string }) => required(value) || max(100)(value),
				onChange: ({ value }: { value: string }) => required(value) || max(100)(value)
			}}
			listeners={{
				onChange: ({ value }: { value: string }) => handleNameChange(value)
			}}
		>
			{#snippet children(field: AnyFieldApi)}
				<TextInput
					label={common_name()}
					id="share-name-{index}"
					{field}
					placeholder={shares_namePlaceholder()}
					required
				/>
			{/snippet}
		</form.Field>
	</div>

	<!-- Share URL / Embed Code — directly below name. Hidden until the share has been
	     saved, since the URL can't resolve until the backend has a persisted record. -->
	{#if isSaved}
		<div class="space-y-3">
			<div>
				<span class="text-secondary mb-1 block text-sm font-medium">{shares_shareUrl()}</span>
				<CodeContainer
					language="bash"
					expandable={false}
					code={generateShareUrl(share.id, themeParam)}
				/>
			</div>
			<div class="space-y-2">
				<span class="text-secondary mb-1 block text-sm font-medium">{shares_embedCode()}</span>
				{#if !hasEmbedsFeature}
					<InlineInfo title={shares_embedsRequirePlan()} body={shares_upgradeForEmbeds()} />
					<div class="mt-2">
						<UpgradeButton feature="embeds" surface="share_panel" />
					</div>
				{:else}
					<CodeContainer
						language="html"
						expandable={false}
						code={generateEmbedCode(share.id, 800, 600, themeParam)}
					/>
				{/if}
			</div>
		</div>
	{:else}
		<InlineInfo
			title={shares_urlAvailableAfterSaveTitle()}
			body={shares_urlAvailableAfterSaveBody()}
		/>
	{/if}

	<!-- Topology Views — collapsible -->
	<CollapsibleCard title={shares_topologyViews()} expanded={false}>
		<div class="space-y-2">
			<ListManager
				label={shares_enabledViews()}
				helpText={shares_enabledViewsHelp()}
				emptyMessage={shares_allViewsEnabled()}
				items={enabledViewItems}
				options={availableViewOptions}
				optionDisplayComponent={SimpleOptionDisplay}
				itemDisplayComponent={SimpleOptionDisplay}
				allowAddFromOptions={true}
				allowCreateNew={false}
				allowReorder={false}
				itemClickAction={null}
				allowItemEdit={() => false}
				onAdd={handleAddView}
				onRemove={handleRemoveView}
			/>
		</div>
	</CollapsibleCard>

	<!-- Access Control — collapsible -->
	<CollapsibleCard title={shares_accessControl()} expanded={false}>
		<div class="space-y-3">
			<form.Field name={passwordFieldName}>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={common_password()}
						id="share-password-{index}"
						type="password"
						{field}
						placeholder={shares_passwordPlaceholder()}
						helpText={shares_passwordHelpEdit()}
					/>
				{/snippet}
			</form.Field>

			<div class="grid grid-cols-2 gap-4">
				<form.Field name={expiresAtFieldName}>
					{#snippet children(field: AnyFieldApi)}
						<DateInput
							{field}
							label={shares_expirationDate()}
							id="expires-at-{index}"
							helpText={shares_expirationHelp()}
						/>
					{/snippet}
				</form.Field>
				<div class="flex items-center">
					<form.Field
						name={isEnabledFieldName}
						listeners={{
							onChange: ({ value }: { value: boolean }) => handleEnabledChange(value)
						}}
					>
						{#snippet children(field: AnyFieldApi)}
							<Checkbox
								label={common_enabled()}
								id="is-enabled-{index}"
								{field}
								helpText={shares_enabledHelp()}
							/>
						{/snippet}
					</form.Field>
				</div>
			</div>

			<form.Field name={allowedDomainsFieldName}>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={shares_allowedEmbedDomains()}
						id="allowed-domains-{index}"
						{field}
						placeholder={shares_allowedDomainsPlaceholder()}
						helpText={shares_allowedDomainsHelp()}
					/>
				{/snippet}
			</form.Field>
		</div>
	</CollapsibleCard>

	<!-- Display Options — collapsible -->
	<CollapsibleCard title={shares_displayOptions()} expanded={false}>
		<div class="space-y-3">
			<form.Field name={showZoomControlsFieldName}>
				{#snippet children(field: AnyFieldApi)}
					<Checkbox label={shares_showZoomControls()} id="show-zoom-controls-{index}" {field} />
				{/snippet}
			</form.Field>
			<form.Field name={showInspectPanelFieldName}>
				{#snippet children(field: AnyFieldApi)}
					<Checkbox label={shares_showInspectPanel()} id="show-inspect-panel-{index}" {field} />
				{/snippet}
			</form.Field>
			<form.Field name={showExportButtonFieldName}>
				{#snippet children(field: AnyFieldApi)}
					<Checkbox label={shares_showExportButton()} id="show-export-button-{index}" {field} />
				{/snippet}
			</form.Field>
			<form.Field name={showMinimapFieldName}>
				{#snippet children(field: AnyFieldApi)}
					<Checkbox label={topology_showMinimap()} id="show-minimap-{index}" {field} />
				{/snippet}
			</form.Field>
			<div>
				<span class="text-secondary mb-1 block text-sm font-medium">{common_theme()}</span>
				<div class="flex gap-2">
					<button
						type="button"
						class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm {shareTheme ===
						'default'
							? 'btn-primary'
							: 'btn-secondary'}"
						onclick={() => (shareTheme = 'default')}
					>
						<Monitor class="h-3.5 w-3.5" />
						{shares_shareThemeDefault()}
					</button>
					<button
						type="button"
						class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm {shareTheme === 'light'
							? 'btn-primary'
							: 'btn-secondary'}"
						onclick={() => (shareTheme = 'light')}
					>
						<Sun class="h-3.5 w-3.5" />
						{shares_shareThemeLight()}
					</button>
					<button
						type="button"
						class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm {shareTheme === 'dark'
							? 'btn-primary'
							: 'btn-secondary'}"
						onclick={() => (shareTheme = 'dark')}
					>
						<Moon class="h-3.5 w-3.5" />
						{shares_shareThemeDark()}
					</button>
				</div>
			</div>
			<span class="text-secondary block text-sm font-medium">{shares_embedDimensions()}</span>
			<div class="grid grid-cols-2 gap-4">
				<form.Field name={embedWidthFieldName}>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_width()}
							id="embed-width-{index}"
							type="number"
							{field}
							placeholder="800"
						/>
					{/snippet}
				</form.Field>
				<form.Field name={embedHeightFieldName}>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_height()}
							id="embed-height-{index}"
							type="number"
							{field}
							placeholder="600"
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>
	</CollapsibleCard>
</div>
