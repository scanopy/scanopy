<script lang="ts">
	/**
	 * Shared component for API key generation and rotation
	 * Used by both daemon API keys and user API keys
	 */
	import CodeContainer from '#lib/shared/components/data/CodeContainer.svelte';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import { RotateCcwKey } from '@lucide/svelte';
	import {
		apiKeys_rotateKey,
		apiKeys_rotateWarningBody,
		apiKeys_rotateWarningTitle,
		apiKeys_saveKeyNowBody,
		apiKeys_saveKeyNowTitle,
		common_generateKey,
		common_generating,
		common_pressGenerateKey
	} from '#lib/paraglide/messages.js';

	interface Props {
		/** The generated key string to display (null if not yet generated) */
		generatedKey: string | null;
		/** Whether this is editing an existing key (shows rotate UI) or creating new */
		isEditing: boolean;
		/** Whether a generation/rotation operation is in progress */
		loading?: boolean;
		/** Callback to generate a new key */
		onGenerate: () => void | Promise<void>;
		/** Callback to rotate an existing key */
		onRotate: () => void | Promise<void>;
	}

	let { generatedKey, isEditing, loading = false, onGenerate, onRotate }: Props = $props();

	function handleClick() {
		if (isEditing) {
			onRotate();
		} else {
			onGenerate();
		}
	}

	let buttonText = $derived(
		loading ? common_generating() : isEditing ? apiKeys_rotateKey() : common_generateKey()
	);
</script>

<div class="space-y-3">
	{#if !generatedKey && isEditing}
		<InlineWarning title={apiKeys_rotateWarningTitle()} body={apiKeys_rotateWarningBody()} />
	{/if}

	{#if generatedKey}
		<InlineWarning title={apiKeys_saveKeyNowTitle()} body={apiKeys_saveKeyNowBody()} />
	{/if}

	<div class="flex items-start gap-2">
		<button
			type="button"
			class="btn-primary shrink-0 self-stretch"
			onclick={handleClick}
			disabled={loading}
		>
			<RotateCcwKey />
			<span>{buttonText}</span>
		</button>

		<div class="flex-1">
			<CodeContainer
				language="bash"
				expandable={false}
				code={generatedKey ? generatedKey : common_pressGenerateKey()}
			/>
		</div>
	</div>
</div>
