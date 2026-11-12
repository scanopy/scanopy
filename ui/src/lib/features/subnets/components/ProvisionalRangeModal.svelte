<!--
	What to do about a range Scanopy assumed.

	The badge said a range was a guess and stopped there, which left the operator with a question and
	no answer to it. These are the four things that actually resolve one, and they are offered
	together because choosing between them is the decision: two settle the range itself, and two
	settle the reason it had to be guessed — that nothing can reach the segment.

	Confirm is the only one that acts here. The rest hand off to the flow that already owns them
	rather than growing a second copy of it: the subnet editor for a correction, the daemon modal for
	a deployment, and the docs for pointing an existing daemon at a segment it can route to.
-->
<script lang="ts">
	import { Check, Edit, Merge } from '@lucide/svelte';

	import ConfirmEntityModal, {
		type ConfirmAction
	} from '#lib/shared/components/confirmation/ConfirmEntityModal.svelte';
	import { openModal } from '#lib/shared/stores/modal-registry.ts';
	import { entities } from '#lib/shared/stores/metadata.ts';
	import type { Subnet, SubnetResponse } from '../types/base';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import {
		subnets_confirmRange,
		subnets_confirmRangeDetail,
		subnets_correctRange,
		subnets_correctRangeDetail,
		subnets_deployDaemonHere,
		subnets_deployDaemonHereDetail,
		subnets_mergeInto,
		subnets_mergeIntoDetail,
		subnets_rangeEvidenceAddresses,
		subnets_resolveRange,
		subnets_resolveRangeIntro
	} from '#lib/paraglide/messages.js';

	interface Props {
		subnet: SubnetResponse | null;
		/**
		 * A live subnet with a settled range that already covers this one, if there is one. Offered
		 * as a merge: containment means every address fits, so folding them loses nothing.
		 */
		coveredBy?: Subnet | null;
		isOpen?: boolean;
		/** Marks the range confirmed. Owned by the tab, which holds the mutation. */
		onConfirm: (subnet: Subnet) => Promise<void> | void;
		/** Opens the subnet editor on this subnet. */
		onCorrect: (subnet: Subnet) => void;
		/** Folds this subnet into the one that covers it. */
		onMerge: (subnet: Subnet, into: Subnet) => Promise<void> | void;
		onClose: () => void;
	}

	let {
		subnet,
		coveredBy = null,
		isOpen = false,
		onConfirm,
		onCorrect,
		onMerge,
		onClose
	}: Props = $props();

	/**
	 * From the entity metadata rather than picked here, so a daemon is drawn with the same icon in
	 * this modal as everywhere else it appears and a change to it reaches all of them at once.
	 */
	const DaemonIcon = entities.getIconComponent('Daemon');

	/**
	 * Where the range came from, carried on the subnet itself: the devices that saw the far ends and
	 * what those far ends published. Absent for a range no scan created.
	 */
	let rangeEvidence = $derived(subnet?.range_evidence ?? null);
	let published = $derived(
		rangeEvidence ? [...rangeEvidence.addresses, ...rangeEvidence.sys_names] : []
	);

	let actions = $derived.by<ConfirmAction[]>(() => {
		if (!subnet) return [];
		const current = subnet;
		const options: ConfirmAction[] = [
			{
				id: 'confirm',
				title: subnets_confirmRange(),
				description: subnets_confirmRangeDetail(),
				icon: Check,
				color: 'Green',
				run: () => void onConfirm(current)
			},
			{
				id: 'correct',
				title: subnets_correctRange(),
				description: subnets_correctRangeDetail(),
				icon: Edit,
				color: 'Blue',
				run: () => onCorrect(current)
			}
		];

		// Only where a measured range already covers this one. Without that there is nothing to fold
		// into, and the question is still how to reach the segment at all.
		if (coveredBy) {
			const into = coveredBy;
			options.push({
				id: 'merge',
				title: subnets_mergeInto({ cidr: into.cidr }),
				description: subnets_mergeIntoDetail(),
				icon: Merge,
				color: 'Violet',
				run: () => void onMerge(current, into)
			});
		}

		options.push({
			id: 'deploy-daemon',
			title: subnets_deployDaemonHere(),
			description: subnets_deployDaemonHereDetail(),
			icon: DaemonIcon,
			color: 'Indigo',
			// The daemon modal lives in the daemons tab, so the hash has to move with it.
			run: () => {
				onClose();
				window.location.hash = 'daemons';
				openModal('create-daemon');
			}
		});

		return options;
	});
</script>

<ConfirmEntityModal
	name="provisional-range"
	entityId={subnet?.id}
	title={subnets_resolveRange()}
	intro={subnet ? subnets_resolveRangeIntro({ cidr: subnet.cidr }) : ''}
	{actions}
	{isOpen}
	{onClose}
	reportedByHostIds={rangeEvidence?.seen_by_host_ids ?? []}
	sourceDiscoveryId={rangeEvidence?.discovery_id ?? null}
>
	{#snippet evidence()}
		{#if published.length > 0}
			<div class="flex flex-wrap items-center gap-2">
				<span class="text-secondary">{subnets_rangeEvidenceAddresses()}</span>
				{#each published as label (label)}
					<Tag {label} color={null} />
				{/each}
			</div>
		{/if}
	{/snippet}
</ConfirmEntityModal>
