<!--
	What to do about an entity Scanopy only inferred: an assumed subnet range, or a host another
	device reported.

	One layout for every unconfirmed entity, so confirming a subnet and confirming a host read the
	same: why it is unconfirmed, the evidence behind it, then the actions that settle it. Each
	entity type supplies its own actions and evidence; the badge that opens this is the shared
	Unconfirmed tag (`confirmation.ts`).
-->
<script lang="ts" module>
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import type { Color } from '#lib/shared/utils/styling.ts';

	export interface ConfirmAction {
		id: string;
		title: string;
		description: string;
		icon: IconComponent;
		color: Color;
		run: () => void;
	}
</script>

<script lang="ts">
	import type { Snippet } from 'svelte';
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import EntityTag from '#lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '#lib/shared/components/data/types.ts';
	import { createColorHelper } from '#lib/shared/utils/styling.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import { openModal } from '#lib/shared/stores/modal-registry.ts';
	import { hostDisplayName } from '#lib/features/hosts/host-display-name.ts';
	import { useHostsByIds } from '#lib/features/hosts/queries.ts';
	import { common_openScanThatFoundIt, common_reportedBy } from '#lib/paraglide/messages.js';

	interface Props {
		name: string;
		entityId?: string;
		title: string;
		intro: string;
		actions: ConfirmAction[];
		isOpen?: boolean;
		onClose: () => void;
		/** The devices whose report the entity is known from, drawn as host links. */
		reportedByHostIds?: string[];
		/** The scan that inferred the entity, opened by the link under the evidence. */
		sourceDiscoveryId?: string | null;
		/** Further evidence particular to the entity type, after who reported it. */
		evidence?: Snippet;
	}

	let {
		name,
		entityId,
		title,
		intro,
		actions,
		isOpen = false,
		onClose,
		reportedByHostIds = [],
		sourceDiscoveryId = null,
		evidence
	}: Props = $props();

	// A reporter deleted since resolves to nothing and is left out.
	const reportersQuery = useHostsByIds(() => reportedByHostIds);
	let reporters = $derived(reportersQuery.data ?? []);

	function openSourceScan(discoveryId: string) {
		onClose();
		window.location.hash = 'discovery-history';
		openModal('discovery-history-detail', { id: discoveryId });
	}
</script>

<GenericModal {name} {entityId} {title} {isOpen} {onClose} size="md">
	<!-- `modal-content` supplies no padding of its own, so a body has to bring the same
	     scrollable, padded container every other modal wraps its content in. -->
	<div class="flex min-h-0 flex-1 flex-col">
		<div class="flex-1 overflow-auto p-6">
			<div class="space-y-4">
				<p class="text-sm text-secondary">{intro}</p>

				{#if reporters.length > 0 || evidence || sourceDiscoveryId}
					<div class="space-y-2 text-sm">
						{#if reporters.length > 0}
							<div class="flex flex-wrap items-center gap-2">
								<span class="text-secondary">{common_reportedBy()}</span>
								{#each reporters as reporter (reporter.id)}
									<EntityTag
										entityRef={entityRef('Host', reporter.id, reporter)}
										label={hostDisplayName(reporter)}
										color={entities.getColorHelper('Host').color}
									/>
								{/each}
							</div>
						{/if}
						{@render evidence?.()}
						{#if sourceDiscoveryId}
							{@const discoveryId = sourceDiscoveryId}
							<button
								type="button"
								class="text-sm text-link hover:underline"
								onclick={() => openSourceScan(discoveryId)}
							>
								{common_openScanThatFoundIt()}
							</button>
						{/if}
					</div>
				{/if}

				<div class="flex flex-col gap-3">
					{#each actions as action (action.id)}
						{@const colors = createColorHelper(action.color)}
						<button onclick={() => action.run()} class="card w-full text-left">
							<div class="flex items-center gap-3">
								<div
									class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {colors.bg}"
								>
									<action.icon class="h-5 w-5 {colors.icon}" />
								</div>
								<div class="min-w-0 flex-1">
									<p class="text-sm font-medium text-primary">{action.title}</p>
									<p class="text-xs text-secondary">{action.description}</p>
								</div>
							</div>
						</button>
					{/each}
				</div>
			</div>
		</div>
	</div>
</GenericModal>
