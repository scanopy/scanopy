/**
 * CSV Export utility for downloading filtered entity data
 */

import { getServerUrl } from '#lib/api/client.ts';
import { pushError } from '#lib/shared/stores/feedback.ts';
import { common_csvExportUnsupported, common_exportFailed } from '#lib/paraglide/messages.js';
import { entityToExportPath, type EntityDiscriminants } from '#lib/api/entities.ts';

/**
 * Parameters for CSV export - matches backend query parameters
 */
export interface CsvExportParams {
	tag_ids?: string[];
	order_by?: string;
	order_direction?: 'asc' | 'desc';
	// Entity-specific filters can be added via index signature. Booleans are
	// here because an export mirrors the list's filters, and some of those are
	// boolean (a host's `hidden` flag); they are stringified on the way out.
	[key: string]: string | string[] | boolean | boolean[] | undefined;
}

/**
 * Put an export's params on its URL. A list's own query (`URLSearchParams`) is already in the
 * endpoint's names, so an export mirrors exactly what the list requested.
 */
function appendExportParams(url: URL, params: CsvExportParams | URLSearchParams) {
	if (params instanceof URLSearchParams) {
		params.forEach((value, key) => url.searchParams.append(key, value));
		return;
	}
	for (const [key, value] of Object.entries(params)) {
		if (value === undefined) continue;
		if (Array.isArray(value)) {
			value.forEach((v) => url.searchParams.append(key, String(v)));
		} else {
			url.searchParams.set(key, String(value));
		}
	}
}

/**
 * Download CSV export for an entity with the given filter parameters.
 * Uses fetch with credentials to handle authentication.
 */
export async function downloadCsv(
	entityType: EntityDiscriminants,
	params: CsvExportParams | URLSearchParams
): Promise<void> {
	const apiPath = entityToExportPath[entityType];
	if (!apiPath) {
		pushError(common_csvExportUnsupported({ entityType }));
		return;
	}

	const baseUrl = getServerUrl();
	const url = new URL(`/api/v1/${apiPath}/export/csv`, baseUrl);

	appendExportParams(url, params);

	const response = await fetch(url.toString(), {
		method: 'GET',
		credentials: 'include'
	});

	if (!response.ok) {
		pushError(common_exportFailed({ error: response.statusText }));
		throw new Error(`Export failed: ${response.statusText}`);
	}

	// Trigger download
	const blob = await response.blob();
	const blobUrl = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = blobUrl;
	link.download = `${apiPath.replace('/', '-')}.csv`;
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(blobUrl);
}

/**
 * Download topology export in Mermaid or Confluence format.
 */
export async function downloadTopologyExport(
	topologyId: string,
	format: 'mermaid' | 'confluence',
	view?: string
): Promise<void> {
	const baseUrl = getServerUrl();
	const url = new URL(`/api/v1/topologies/${topologyId}/export/${format}`, baseUrl);
	if (view) url.searchParams.set('view', view);

	const response = await fetch(url.toString(), {
		method: 'GET',
		credentials: 'include'
	});

	if (!response.ok) {
		pushError(common_exportFailed({ error: response.statusText }));
		throw new Error(`Export failed: ${response.statusText}`);
	}

	const blob = await response.blob();
	const blobUrl = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = blobUrl;
	const date = new Date().toISOString().split('T')[0];
	const ext = format === 'mermaid' ? 'mmd' : 'txt';
	link.download = `scanopy-topology-${date}.${ext}`;
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(blobUrl);
}

/**
 * Download ZIP export for hosts with all children.
 * Returns a zip containing hosts.csv, interfaces.csv, ports.csv, services.csv, interfaces.csv
 */
export async function downloadHostsZip(params: CsvExportParams | URLSearchParams): Promise<void> {
	const baseUrl = getServerUrl();
	const url = new URL('/api/v1/hosts/export/zip', baseUrl);

	appendExportParams(url, params);

	const response = await fetch(url.toString(), {
		method: 'GET',
		credentials: 'include'
	});

	if (!response.ok) {
		pushError(common_exportFailed({ error: response.statusText }));
		throw new Error(`Export failed: ${response.statusText}`);
	}

	// Trigger download
	const blob = await response.blob();
	const blobUrl = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = blobUrl;
	link.download = 'hosts-export.zip';
	document.body.appendChild(link);
	link.click();
	document.body.removeChild(link);
	URL.revokeObjectURL(blobUrl);
}
