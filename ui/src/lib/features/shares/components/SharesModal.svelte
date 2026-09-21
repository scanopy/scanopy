<script lang="ts">
	import { createForm } from '@tanstack/svelte-form';
	import { validateForm } from '$lib/shared/components/forms/form-context';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import ModalHeaderIcon from '$lib/shared/components/layout/ModalHeaderIcon.svelte';
	import ListConfigEditor from '$lib/shared/components/forms/selection/ListConfigEditor.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import EntityConfigEmpty from '$lib/shared/components/forms/EntityConfigEmpty.svelte';
	import ShareConfigPanel from './ShareConfigPanel.svelte';
	import {
		ShareDisplay,
		type ShareDisplayContext
	} from '$lib/shared/components/forms/selection/display/ShareDisplay.svelte';
	import { Share2 } from 'lucide-svelte';
	import { v4 as uuidv4 } from 'uuid';
	import type { Share } from '../types/base';
	import { createEmptyShare, defaultShareOptions } from '../types/base';
	import {
		useSharesQuery,
		useCreateShareMutation,
		useDeleteShareMutation,
		useUpdateShareMutation
	} from '../queries';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useTopologiesQuery } from '$lib/features/topology/queries';
	import { billingPlans, entities } from '$lib/shared/stores/metadata';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import UpgradeButton from '$lib/shared/components/UpgradeButton.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import {
		common_close,
		common_noEntitySelected,
		common_save,
		common_saving,
		common_share,
		common_shares,
		common_validation_entityField,
		shares_manageShares,
		shares_noSharesSubtitle,
		shares_noSharesYet,
		shares_selectToEdit,
		shares_snapshotShareInfoTitle,
		shares_snapshotShareInfoBody
	} from '$lib/paraglide/messages';

	let {
		isOpen = false,
		onClose,
		topologyId = '',
		networkId = '',
		name = undefined,
		topologyDisplayName = '',
		isSnapshotView = false
	}: {
		isOpen?: boolean;
		onClose: () => void;
		topologyId?: string;
		networkId?: string;
		/** Modal-registry slug; threaded into GenericModal. */
		name?: string;
		/** Display name used in the modal title. Topology rows no longer carry
		 *  a name field, so callers pass it explicitly (network name for live,
		 *  formatted snapshot timestamp otherwise). */
		topologyDisplayName?: string;
		/** True when the user opened this while viewing a snapshot. A share always
		 *  renders the live view, so we surface an inline notice. */
		isSnapshotView?: boolean;
	} = $props();

	// Queries
	const sharesQuery = useSharesQuery();
	const currentUserQuery = useCurrentUserQuery();
	let currentUser = $derived(currentUserQuery.data);

	// Keep the topologies query around so the modal verifies the referenced
	// topology exists; title comes from `topologyDisplayName` directly.
	useTopologiesQuery();
	let modalTitle = $derived(shares_manageShares({ name: topologyDisplayName }));

	const organizationQuery = useOrganizationQuery();
	let hasShareViews = $derived.by(() => {
		const org = organizationQuery.data;
		if (!org?.plan) return true;
		return billingPlans.getMetadata(org.plan.type).features.share_views;
	});

	// Local state — single source of truth for the list during the modal's lifetime.
	let sharesData: Share[] = $state([]);
	let originalShareIds = $state(new Set<string>());
	let hydrated = $state(false);

	// Mutations
	const createShareMutation = useCreateShareMutation();
	const deleteShareMutation = useDeleteShareMutation();
	const updateShareMutation = useUpdateShareMutation();

	let saving = $state(false);

	function toFormEntry(s: Share) {
		return {
			name: s.name || '',
			// Mirrors the backend: when a password is set the response carries the
			// redaction sentinel in `s.password`. Writing it back unchanged tells the
			// backend to preserve; typing over it sends a new plaintext.
			password: s.password ?? '',
			allowed_domains: s.allowed_domains?.join(', ') || '',
			expires_at: s.expires_at || '',
			is_enabled: s.is_enabled ?? true,
			show_zoom_controls: s.options?.show_zoom_controls ?? defaultShareOptions.show_zoom_controls,
			show_inspect_panel: s.options?.show_inspect_panel ?? defaultShareOptions.show_inspect_panel,
			show_export_button: s.options?.show_export_button ?? defaultShareOptions.show_export_button,
			show_minimap: s.options?.show_minimap ?? defaultShareOptions.show_minimap,
			embed_width: '800',
			embed_height: '600'
		};
	}

	// Parent-owned form — starts empty; hydration happens in the isOpen effect.
	const form = createForm(() => ({
		defaultValues: { shares: [] as ReturnType<typeof toFormEntry>[] },
		onSubmit: async () => {
			// Submission handled by handleSave
		}
	}));

	// One-time hydration on open: seed sharesData + form + snapshot of server IDs.
	// Reset when the modal closes so the next open re-hydrates from the latest cache.
	$effect(() => {
		if (!isOpen) {
			hydrated = false;
			return;
		}
		if (hydrated || sharesQuery.isPending) return;
		const queryShares = (sharesQuery.data ?? []).filter((s) => s.topology_id === topologyId);
		sharesData = queryShares;
		originalShareIds = new Set(queryShares.map((s) => s.id));
		form.reset({ shares: queryShares.map(toFormEntry) });
		hydrated = true;
	});

	function handleShareChange(updatedShare: Share, index: number) {
		const updated = [...sharesData];
		updated[index] = updatedShare;
		sharesData = updated;
	}

	function handleCreateNew() {
		const newShare: Share = {
			...createEmptyShare(topologyId, networkId),
			id: uuidv4(),
			created_by: currentUser?.id ?? ''
		};
		sharesData = [...sharesData, newShare];
		// Append to the form array in place so in-flight edits on existing entries
		// (typed passwords, toggled checkboxes) survive the add.
		const current = form.state.values.shares ?? [];
		form.setFieldValue('shares', [...current, toFormEntry(newShare)]);
	}

	function handleRemove(index: number) {
		sharesData = sharesData.filter((_, i) => i !== index);
		const current = form.state.values.shares ?? [];
		form.setFieldValue(
			'shares',
			current.filter((_, i) => i !== index)
		);
	}

	// Resolve field paths to human-readable names for validation errors
	function resolveFieldName(fieldPath: string): string {
		const match = fieldPath.match(/^shares\[(\d+)]\.(.+)$/);
		if (match) {
			const index = parseInt(match[1]);
			const field = match[2].replace(/_/g, ' ');
			const share = sharesData[index];
			const shareName = share?.name || `Share ${index + 1}`;
			return common_validation_entityField({ name: shareName, field });
		}
		return fieldPath.replace(/_/g, ' ');
	}

	function buildShareFromFormValue(share: Share, v: ReturnType<typeof toFormEntry>): Share {
		return {
			id: share.id,
			name: v.name?.trim() || '',
			topology_id: share.topology_id,
			network_id: share.network_id,
			created_by: currentUser?.id || share.created_by,
			allowed_domains: v.allowed_domains?.trim()
				? v.allowed_domains
						.split(',')
						.map((d) => d.trim())
						.filter(Boolean)
				: null,
			expires_at: v.expires_at || null,
			is_enabled: v.is_enabled,
			// Empty string on a fresh share = "no password"; on an edited share =
			// "clear the existing password". The backend distinguishes via the
			// redaction sentinel: untouched entries still carry it, so the server
			// preserves. We leave the string as-is here.
			password: v.password,
			options: {
				show_zoom_controls: v.show_zoom_controls,
				show_inspect_panel: v.show_inspect_panel,
				show_export_button: v.show_export_button,
				show_minimap: v.show_minimap
			},
			enabled_views: share.enabled_views
		} as Share;
	}

	async function handleSave() {
		const isValid = await validateForm(form, undefined, resolveFieldName);
		if (!isValid) return;

		saving = true;
		try {
			const formShares = (form.state.values.shares ?? []) as ReturnType<typeof toFormEntry>[];
			const currentIds = new Set(sharesData.map((s) => s.id));

			// Deletes: in snapshot but not in current
			for (const id of originalShareIds) {
				if (!currentIds.has(id)) {
					await deleteShareMutation.mutateAsync(id);
				}
			}

			// Creates + updates, in list order
			for (let i = 0; i < sharesData.length; i++) {
				const share = sharesData[i];
				const v = formShares[i];
				if (!v) continue;
				const shareData = buildShareFromFormValue(share, v);

				if (originalShareIds.has(share.id)) {
					await updateShareMutation.mutateAsync({
						id: share.id,
						request: { share: shareData }
					});
				} else {
					await createShareMutation.mutateAsync({ share: shareData });
				}
			}
			// Save stays open: trigger a fresh hydration so originalShareIds picks up the
			// newly-saved IDs (URLs/embed then become visible for those shares) and form
			// state is reset from the now-canonical query cache.
			hydrated = false;
		} catch {
			// The API client reports the failure.
		} finally {
			saving = false;
		}
	}
</script>

<GenericModal
	{isOpen}
	title={modalTitle}
	{name}
	size="xl"
	{onClose}
	showCloseButton={true}
	fixedHeight={true}
>
	{#snippet headerIcon()}
		<ModalHeaderIcon Icon={Share2} color={entities.getColorHelper('Share').color} />
	{/snippet}

	{#if isSnapshotView}
		<div class="px-6 pb-2 pt-4">
			<InlineInfo title={shares_snapshotShareInfoTitle()} body={shares_snapshotShareInfoBody()} />
		</div>
	{/if}

	{#if !hasShareViews}
		<div class="flex min-h-0 flex-1 flex-col items-center justify-center p-6">
			<EmptyState title={shares_noSharesYet()} subtitle={shares_noSharesSubtitle()}>
				<UpgradeButton feature="share_views" surface="shares_modal" />
			</EmptyState>
		</div>
	{:else}
		<div class="flex min-h-0 flex-1 flex-col">
			<ListConfigEditor
				items={sharesData}
				loading={sharesQuery.isPending}
				onReorder={() => {}}
				onChange={() => {}}
			>
				<svelte:fragment
					slot="list"
					let:items
					let:onEdit
					let:highlightedIndex
					let:onMoveUp
					let:onMoveDown
				>
					<ListManager
						label={common_shares()}
						emptyMessage={shares_noSharesSubtitle()}
						allowAddFromOptions={false}
						allowCreateNew={true}
						allowReorder={false}
						itemClickAction="edit"
						optionDisplayComponent={ShareDisplay}
						itemDisplayComponent={ShareDisplay}
						getItemContext={() => ({}) as ShareDisplayContext}
						{items}
						onCreateNew={handleCreateNew}
						onRemove={handleRemove}
						{onEdit}
						{onMoveUp}
						{onMoveDown}
						{highlightedIndex}
					/>
				</svelte:fragment>

				<svelte:fragment slot="config" let:selectedItem let:selectedIndex>
					<!-- Render all config panels to register form fields, only show selected -->
					{#each sharesData as share, index (`${share.id}-${index}`)}
						<div class:hidden={selectedIndex !== index}>
							<ShareConfigPanel
								{share}
								{index}
								{form}
								isSaved={originalShareIds.has(share.id)}
								onChange={(updatedShare) => handleShareChange(updatedShare, index)}
							/>
						</div>
					{/each}

					{#if !selectedItem}
						<EntityConfigEmpty
							title={common_noEntitySelected({ entity: common_share() })}
							subtitle={shares_selectToEdit()}
						/>
					{/if}
				</svelte:fragment>
			</ListConfigEditor>
		</div>
	{/if}

	{#snippet footer()}
		<div class="modal-footer">
			<div class="flex items-center justify-end gap-3">
				<button type="button" onclick={onClose} class="btn-secondary">
					{common_close()}
				</button>
				{#if sharesData.length > 0 || originalShareIds.size > 0}
					<button type="button" disabled={saving} onclick={handleSave} class="btn-primary">
						{saving ? common_saving() : common_save()}
					</button>
				{/if}
			</div>
		</div>
	{/snippet}
</GenericModal>
