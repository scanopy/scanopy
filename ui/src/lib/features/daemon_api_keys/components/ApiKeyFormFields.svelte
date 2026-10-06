<script lang="ts">
	/**
	 * The editable fields of an API key, plus its generate/rotate control.
	 *
	 * Extracted from ApiKeyModal so the daemon management modal can offer the same key
	 * editing inside a tab — the modal shell, footer and form lifecycle can't nest, but
	 * the fields can. The owning component supplies the TanStack form.
	 */
	import type { ApiKeyForm } from '../form';
	import { required, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import DateInput from '$lib/shared/components/forms/input/DateInput.svelte';
	import SelectSite from '$lib/features/sites/components/SelectSite.svelte';
	import Checkbox from '$lib/shared/components/forms/input/Checkbox.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import ApiKeyGenerator from '$lib/shared/components/api-keys/ApiKeyGenerator.svelte';
	import {
		common_apiKeyNameHelp,
		common_enableApiKey,
		common_expirationDateOptional,
		common_expirationNeverHelp,
		common_name,
		daemonApiKeys_enableApiKeyHelp,
		daemonApiKeys_namePlaceholder
	} from '$lib/paraglide/messages';

	interface Props {
		form: ApiKeyForm;
		isEditing: boolean;
		generatedKey: string | null;
		loading: boolean;
		onGenerate: () => void;
		onRotate: () => void;
		/** Hide the site selector where the site is fixed by the owning entity. */
		showSite?: boolean;
		/** Hide the name field where it is owned by another entity (e.g. a daemon's key). */
		showName?: boolean;
		/** Hide the tag picker where tags aren't meaningful for the key. */
		showTags?: boolean;
	}

	let {
		form,
		isEditing,
		generatedKey,
		loading,
		onGenerate,
		onRotate,
		showSite = true,
		showName = true,
		showTags = true
	}: Props = $props();

	// Minimum selectable expiry (now), in the local format datetime-local expects.
	function getLocalDateTimeMin(): string {
		const now = new Date();
		const pad = (n: number) => String(n).padStart(2, '0');
		return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T${pad(now.getHours())}:${pad(now.getMinutes())}`;
	}
	const today = getLocalDateTimeMin();
</script>

<div class="space-y-6">
	<div class="space-y-4">
		{#if showName}
			<form.Field
				name="name"
				validators={{
					onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
				}}
			>
				{#snippet children(field)}
					<TextInput
						label={common_name()}
						id="name"
						{field}
						placeholder={daemonApiKeys_namePlaceholder()}
						helpText={common_apiKeyNameHelp()}
						required
					/>
				{/snippet}
			</form.Field>
		{/if}

		{#if showSite}
			<form.Field name="site_id">
				{#snippet children(field)}
					<SelectSite
						selectedSiteId={field.state.value}
						onSiteChange={(id) => field.handleChange(id)}
						disabled={isEditing}
					/>
				{/snippet}
			</form.Field>
		{/if}

		{#if showTags}
			<form.Field name="tags">
				{#snippet children(field)}
					<TagPicker
						selectedTagIds={field.state.value || []}
						onChange={(tags) => field.handleChange(tags)}
					/>
				{/snippet}
			</form.Field>
		{/if}

		<form.Field name="expires_at">
			{#snippet children(field)}
				<DateInput
					label={common_expirationDateOptional()}
					id="expires_at"
					{field}
					helpText={common_expirationNeverHelp()}
					min={today}
				/>
			{/snippet}
		</form.Field>

		<form.Field name="is_enabled">
			{#snippet children(field)}
				<Checkbox
					{field}
					label={common_enableApiKey()}
					helpText={daemonApiKeys_enableApiKeyHelp()}
					id="enableApiKey"
				/>
			{/snippet}
		</form.Field>
	</div>

	<ApiKeyGenerator {generatedKey} {isEditing} {loading} {onGenerate} {onRotate} />
</div>
