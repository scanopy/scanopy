<script lang="ts">
	import GenericModal from '#lib/shared/components/layout/GenericModal.svelte';
	import { FileSpreadsheet, FileArchive } from '@lucide/svelte';
	import {
		downloadCsv,
		downloadHostsZip,
		type CsvExportParams
	} from '#lib/shared/utils/csvExport.ts';
	import {
		hosts_exportChooseFormat,
		hosts_exportCsvDesc,
		hosts_exportCsvTitle,
		hosts_exportTitle,
		hosts_exportZipDesc,
		hosts_exportZipTitle
	} from '#lib/paraglide/messages.js';

	let {
		isOpen = false,
		onClose,
		exportParams = {}
	}: {
		isOpen?: boolean;
		onClose: () => void;
		exportParams?: CsvExportParams | URLSearchParams;
	} = $props();

	let isExporting = $state(false);

	async function handleExportCsv() {
		isExporting = true;
		try {
			await downloadCsv('Host', exportParams);
			onClose();
		} finally {
			isExporting = false;
		}
	}

	async function handleExportZip() {
		isExporting = true;
		try {
			await downloadHostsZip(exportParams);
			onClose();
		} finally {
			isExporting = false;
		}
	}
</script>

<GenericModal title={hosts_exportTitle()} {isOpen} {onClose} size="sm">
	<div class="p-6">
		<p class="mb-4 text-sm text-secondary">{hosts_exportChooseFormat()}</p>

		<div class="space-y-3">
			<button
				onclick={handleExportCsv}
				disabled={isExporting}
				class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
			>
				<FileSpreadsheet class="h-6 w-6 shrink-0 text-tertiary" />
				<div>
					<div class="font-medium text-primary">{hosts_exportCsvTitle()}</div>
					<div class="text-sm text-tertiary">{hosts_exportCsvDesc()}</div>
				</div>
			</button>

			<button
				onclick={handleExportZip}
				disabled={isExporting}
				class="card flex w-full items-start gap-4 p-4 text-left transition-colors hover:border-blue-500/50 disabled:opacity-50"
			>
				<FileArchive class="h-6 w-6 shrink-0 text-tertiary" />
				<div>
					<div class="font-medium text-primary">{hosts_exportZipTitle()}</div>
					<div class="text-sm text-tertiary">{hosts_exportZipDesc()}</div>
				</div>
			</button>
		</div>
	</div>
</GenericModal>
