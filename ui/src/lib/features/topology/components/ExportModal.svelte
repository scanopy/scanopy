<script lang="ts">
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { toPng, toSvg } from 'html-to-image';
	import { useSvelteFlow } from '@xyflow/svelte';
	import {
		FileImage,
		FileCode,
		FileText,
		FileOutput,
		AppWindow,
		Image,
		Workflow,
		Network,
		Sun,
		Moon
	} from '@lucide/svelte';
	import { pushError, pushSuccess } from '$lib/shared/stores/feedback';
	import { activeView, hiddenEdgeTypes } from '../queries';
	import type { RenderableTopology } from '../types/base';
	import { escapeXml } from '../diagram-export/xml';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { billingPlans, containerTypes, views } from '$lib/shared/stores/metadata';
	import { EXTERNAL_TITLE_OFFSET } from '../container-render-data';
	import { isExporting } from '../interactions';
	import {
		topology_createdUsing,
		topology_export,
		topology_exportComplete,
		topology_exportFailed,
		topology_exportPng,
		topology_exportPngDesc,
		topology_exportSvg,
		topology_exportSvgDesc,
		topology_exportMermaid,
		topology_exportMermaidDesc,
		topology_exportConfluence,
		topology_exportConfluenceDesc,
		topology_exportPdf,
		topology_exportPdfDesc,
		topology_exportHtml,
		topology_exportHtmlDesc,
		topology_exportDrawio,
		topology_exportDrawioDesc,
		topology_exportVsdx,
		topology_exportVsdxDesc,
		topology_exportHeaderTitle,
		topology_exportHeaderSnapshot,
		topology_exportTheme,
		topology_exportIncludeHeader,
		topology_flowNotFound,
		topology_noNodesToExport
	} from '$lib/paraglide/messages';
	import { getResolvedTheme } from '$lib/shared/stores/theme.svelte';
	import { common_light, common_dark, common_export } from '$lib/paraglide/messages';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { downloadTopologyExport } from '$lib/shared/utils/csvExport';
	import GenericModal from '$lib/shared/components/layout/GenericModal.svelte';
	import type { ModalAction } from '$lib/shared/components/layout/modalActions';
	import { closeModal } from '$lib/shared/stores/modal-registry';
	import { triggerUpgrade } from '$lib/features/billing/trigger-upgrade';
	import { upgradeTagProps } from '$lib/features/billing/upgrade-tag';
	import type { UpgradeFeature } from '$lib/shared/stores/metadata';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		SimpleOptionDisplay,
		type SimpleOption
	} from '$lib/shared/components/forms/selection/display/SimpleOptionDisplay';
	import type { ExportFeatures } from '$lib/features/shares/types/base';

	// Same fallback used by LogoIcon.svelte when a logo fails to load
	const LOGO_FALLBACK =
		'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQiIGhlaWdodD0iMjQiIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPGNpcmNsZSBjeD0iMTIiIGN5PSIxMiIgcj0iMTAiIHN0cm9rZT0iY3VycmVudENvbG9yIiBzdHJva2Utd2lkdGg9IjIiLz4KPHA=';

	let {
		topologyId,
		topology,
		topologyName = '',
		snapshotTakenAt = null,
		isOpen = $bindable(false),
		isShareView = false,
		exportFeatures = undefined,
		name = undefined
	}: {
		topologyId: string;
		/** The topology as drawn, for the editable diagram exports' text and colours. */
		topology: RenderableTopology;
		/** Site name in the app, share name in a share view. Leads the header and the filename. */
		topologyName?: string;
		/** When a snapshot is shown, its capture time, which dates the header instead of now. */
		snapshotTakenAt?: string | null;
		isOpen: boolean;
		isShareView?: boolean;
		exportFeatures?: ExportFeatures;
		/** Modal-registry slug; threaded into GenericModal so closing clears the registry. */
		name?: string;
	} = $props();

	const { getNodes, getEdges, getInternalNode, getNodesBounds, getViewport, setViewport } =
		useSvelteFlow();

	/**
	 * Close through the registry when the modal is registered: the parent derives `isOpen` from
	 * it, so a local write leaves the registry (and the URL) naming this modal, and the next
	 * `openModal` changes nothing the parent can see.
	 */
	function close() {
		if (name) closeModal();
		else isOpen = false;
	}

	/** Lowercase a site name and reduce non-alphanumerics to single hyphens for safe filenames. */
	function sanitizeForFilename(name: string): string {
		const slug = name
			.toLowerCase()
			.replace(/[^a-z0-9]+/g, '-')
			.replace(/-+/g, '-')
			.replace(/^-|-$/g, '');
		return slug || 'site';
	}

	/** Build the canonical export filename: scanopy-<view>-<site>-<date>.<ext>. */
	function buildExportFilename(ext: string): string {
		const viewToken = views.getMetadata($activeView).slug ?? $activeView;
		const siteSlug = sanitizeForFilename(topologyName);
		const date = new Date().toISOString().split('T')[0];
		return `scanopy-${viewToken}-${siteSlug}-${date}.${ext}`;
	}

	/**
	 * The header every format draws when "Include title and date" is on: site and view, then the
	 * snapshot's capture time when a snapshot is shown, otherwise the export time.
	 */
	function exportHeader(now: Date): { title: string; subtitle: string } {
		const view = views.getName($activeView) || $activeView;
		return {
			title: topologyName ? topology_exportHeaderTitle({ site: topologyName, view }) : view,
			subtitle: snapshotTakenAt
				? topology_exportHeaderSnapshot({ date: formatTimestamp(snapshotTakenAt) })
				: formatTimestamp(now)
		};
	}

	// Recomputed on open, so the preview's time is the time the modal was opened.
	let headerPreview = $derived.by(() => {
		void isOpen;
		return exportHeader(new Date());
	});

	const organizationQuery = useOrganizationQuery();
	let organization = $derived(organizationQuery.data);

	// Feature gate helpers — use exportFeatures (share view) or org plan (authenticated)
	function hasFeature(key: keyof ExportFeatures): boolean {
		if (exportFeatures) return exportFeatures[key] as boolean;
		if (organization?.plan) {
			const features = billingPlans.getMetadata(organization.plan.type).features;
			return (features as unknown as Record<string, boolean>)[key] ?? true;
		}
		return !isShareView;
	}

	// Rasterize at >= 2× the device pixel ratio so node titles stay crisp on
	// hi-DPI displays. A flat `pixelRatio: 2` looks blurry on a 2× Retina
	// screen because it's effectively 1× of what the user sees. Floor of 2
	// keeps non-DPI displays from rasterizing below the previous baseline.
	function effectivePixelRatio(): number {
		const dpr = typeof window !== 'undefined' ? (window.devicePixelRatio ?? 1) : 1;
		return Math.max(2, dpr * 2);
	}

	let hideCreatedWith = $derived(hasFeature('remove_created_with'));
	let hasPngExport = $derived(hasFeature('png_export'));
	let hasSvgExport = $derived(hasFeature('svg_export'));
	let hasMermaidExport = $derived(hasFeature('mermaid_export'));
	let hasConfluenceExport = $derived(hasFeature('confluence_export'));
	let hasPdfExport = $derived(hasFeature('pdf_export'));
	let hasHtmlExport = $derived(hasFeature('html_export'));
	let hasDrawioExport = $derived(hasFeature('drawio_export'));
	let hasVsdxExport = $derived(hasFeature('vsdx_export'));

	let exportTheme = $state<'light' | 'dark'>(getResolvedTheme());
	let selectedFormat = $state<string>('png');
	let includeHeader = $state(false);

	// Format definitions
	const allFormats: {
		value: string;
		label: string;
		description: string;
		icon: SimpleOption['icon'];
		featureKey?: keyof ExportFeatures;
		supportsTheme: boolean;
		supportsHeader: boolean;
	}[] = [
		{
			value: 'png',
			label: topology_exportPng(),
			description: topology_exportPngDesc(),
			icon: Image,
			featureKey: 'png_export',
			supportsTheme: true,
			supportsHeader: true
		},
		{
			value: 'svg',
			label: topology_exportSvg(),
			description: topology_exportSvgDesc(),
			icon: FileImage,
			featureKey: 'svg_export',
			supportsTheme: true,
			supportsHeader: true
		},
		{
			value: 'pdf',
			label: topology_exportPdf(),
			description: topology_exportPdfDesc(),
			icon: FileOutput,
			featureKey: 'pdf_export',
			supportsTheme: true,
			supportsHeader: true
		},
		{
			value: 'html',
			label: topology_exportHtml(),
			description: topology_exportHtmlDesc(),
			icon: AppWindow,
			featureKey: 'html_export',
			supportsTheme: true,
			supportsHeader: true
		},
		{
			value: 'drawio',
			label: topology_exportDrawio(),
			description: topology_exportDrawioDesc(),
			icon: Network,
			featureKey: 'drawio_export',
			supportsTheme: true,
			supportsHeader: true
		},
		{
			value: 'vsdx',
			label: topology_exportVsdx(),
			description: topology_exportVsdxDesc(),
			icon: Workflow,
			featureKey: 'vsdx_export',
			supportsTheme: true,
			supportsHeader: true
		},
		{
			value: 'mermaid',
			label: topology_exportMermaid(),
			description: topology_exportMermaidDesc(),
			icon: FileCode,
			featureKey: 'mermaid_export',
			supportsTheme: false,
			supportsHeader: false
		},
		{
			value: 'confluence',
			label: topology_exportConfluence(),
			description: topology_exportConfluenceDesc(),
			icon: FileText,
			featureKey: 'confluence_export',
			supportsTheme: false,
			supportsHeader: false
		}
	];

	// Feature gate map for reactive access
	const featureGates: Record<string, () => boolean> = {
		png_export: () => hasPngExport,
		svg_export: () => hasSvgExport,
		mermaid_export: () => hasMermaidExport,
		confluence_export: () => hasConfluenceExport,
		pdf_export: () => hasPdfExport,
		html_export: () => hasHtmlExport,
		drawio_export: () => hasDrawioExport,
		vsdx_export: () => hasVsdxExport
	};

	let formatOptions = $derived.by(() => {
		const upgradeTags = [upgradeTagProps()];
		return allFormats
			.filter((f) => {
				// In share view, hide formats the plan doesn't support
				if (isShareView && f.featureKey && !featureGates[f.featureKey]?.()) return false;
				return true;
			})
			.map((f): SimpleOption => ({
				value: f.value,
				label: f.label,
				description: f.description,
				icon: f.icon,
				disabled: f.featureKey ? !featureGates[f.featureKey]?.() : false,
				tags: f.featureKey && !featureGates[f.featureKey]?.() ? upgradeTags : []
			}));
	});

	let selectedFormatDef = $derived(allFormats.find((f) => f.value === selectedFormat));
	// PNG is selected by default, so a plan without it opens onto a format it
	// can't export; the Export button waits for one it can.
	let selectedFormatGated = $derived(
		selectedFormatDef?.featureKey != null && !featureGates[selectedFormatDef.featureKey]?.()
	);

	function handleFormatSelect(value: string) {
		selectedFormat = value;
	}

	function handleDisabledFormatClick(value: string) {
		const format = allFormats.find((f) => f.value === value);
		if (format?.featureKey) {
			handleUpgrade(format.featureKey);
		}
	}

	function handleExport() {
		if (selectedFormatGated) return;
		switch (selectedFormat) {
			case 'png':
				captureImage('png');
				break;
			case 'svg':
				captureImage('svg');
				break;
			case 'pdf':
				handlePdfExport();
				break;
			case 'html':
				handleHtmlExport();
				break;
			case 'mermaid':
				handleMermaidExport();
				break;
			case 'confluence':
				handleConfluenceExport();
				break;
			case 'drawio':
			case 'vsdx':
				handleDiagramExport(selectedFormat);
				break;
		}
	}

	/** Uniform padding (flow units) around the content bounds so the crop isn't flush to nodes. */
	const EXPORT_PADDING = 40;
	/** Image scale the raster formats capture at. */
	const EXPORT_ZOOM = 0.75;
	/**
	 * Image pixels from the top the header block takes, with a gap below it: 12px offset, an 18px
	 * title, a 12px date line and their line heights. Content starts below it so the header never
	 * covers a node.
	 */
	const HEADER_BAND_PX = 66;

	function calculateExportBounds(withHeader: boolean) {
		const nodes = getNodes();

		if (nodes.length === 0) {
			pushError(topology_noNodesToExport());
			return null;
		}

		const flowElement = document.querySelector('.svelte-flow') as HTMLElement;
		if (!flowElement) {
			pushError(topology_flowNotFound());
			return null;
		}

		// Union bounding box of every rendered node/container in absolute flow
		// coords. The hook's getNodesBounds resolves nested container offsets via
		// the internal node lookup, so the crop is tight and symmetric on all sides.
		const bounds = getNodesBounds(nodes);
		// External title pills draw above their container's box, outside the node bounds.
		const contentTop = nodes.reduce((top, node) => {
			const containerType = (node.data as { container_type?: string })?.container_type;
			if (!containerType || containerTypes.getMetadata(containerType).title_style !== 'External')
				return top;
			const y = getInternalNode(node.id)?.internals.positionAbsolute.y ?? node.position.y;
			return Math.min(top, y - EXTERNAL_TITLE_OFFSET);
		}, bounds.y);
		const topPadding = withHeader
			? Math.max(EXPORT_PADDING, HEADER_BAND_PX / EXPORT_ZOOM)
			: EXPORT_PADDING;
		const minX = bounds.x - EXPORT_PADDING;
		const minY = contentTop - topPadding;
		const boundsWidth = bounds.width + EXPORT_PADDING * 2;
		const boundsHeight = bounds.y + bounds.height - contentTop + topPadding + EXPORT_PADDING;
		const targetZoom = EXPORT_ZOOM;
		const imageWidth = Math.round(boundsWidth * targetZoom);
		const imageHeight = Math.round(boundsHeight * targetZoom);
		const boundsCenterX = minX + boundsWidth / 2;
		const boundsCenterY = minY + boundsHeight / 2;
		const x = imageWidth / 2 - boundsCenterX * targetZoom;
		const y = imageHeight / 2 - boundsCenterY * targetZoom;

		return { flowElement, imageWidth, imageHeight, viewport: { x, y, zoom: targetZoom } };
	}

	interface ExportCaptureContext {
		flowElement: HTMLElement;
		imageWidth: number;
		imageHeight: number;
	}

	/**
	 * Fit the flow to the content, apply the export theme and `.hide-for-export`, and run the
	 * callback. `decorate` adds the header and watermark to the DOM, for the picture formats that
	 * capture it; the diagram formats draw their own header and leave it off.
	 */
	async function withExportCapture(
		callback: (ctx: ExportCaptureContext) => Promise<void>,
		{ decorate = true }: { decorate?: boolean } = {}
	): Promise<void> {
		close();

		// Wait for modal to close so it's not captured in the image
		await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

		const bounds = calculateExportBounds(decorate && includeHeader);
		if (!bounds) return;

		const { flowElement, imageWidth, imageHeight, viewport: newViewport } = bounds;
		const originalViewport = getViewport();
		const originalWidth = flowElement.style.width;
		const originalHeight = flowElement.style.height;

		// Temporarily switch theme for export
		const currentTheme = getResolvedTheme();
		const needsThemeSwitch = exportTheme !== currentTheme;
		if (needsThemeSwitch) {
			document.documentElement.classList.toggle('dark', exportTheme === 'dark');
			document.documentElement.style.colorScheme = exportTheme;
		}

		flowElement.style.width = `${imageWidth}px`;
		flowElement.style.height = `${imageHeight}px`;
		setViewport(newViewport, { duration: 0 });
		flowElement.classList.add('hide-for-export');
		isExporting.set(true);

		// Inject header with title and date
		let header = document.createElement('div');
		if (decorate && includeHeader) {
			const { title: exportName, subtitle: exportDate } = exportHeader(new Date());
			const textColor = exportTheme === 'dark' ? 'rgba(255, 255, 255, 0.9)' : 'rgba(0, 0, 0, 0.8)';
			const dateColor = exportTheme === 'dark' ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.4)';
			header.style.cssText = `
				position: absolute;
				top: 12px;
				left: 20px;
				font-family: system-ui;
				pointer-events: none;
				z-index: 9999;
			`;
			const titleEl = document.createElement('div');
			titleEl.textContent = exportName;
			titleEl.style.cssText = `font-size: 18px; font-weight: 600; color: ${textColor};`;
			const dateEl = document.createElement('div');
			dateEl.textContent = exportDate;
			dateEl.style.cssText = `font-size: 12px; color: ${dateColor}; margin-top: 2px;`;
			header.appendChild(titleEl);
			header.appendChild(dateEl);
			flowElement.appendChild(header);
		}

		// Inject watermark
		let watermark = document.createElement('div');
		const watermarkColor =
			exportTheme === 'dark' ? 'rgba(255, 255, 255, 0.5)' : 'rgba(0, 0, 0, 0.3)';

		if (decorate && !hideCreatedWith) {
			watermark = document.createElement('div');
			watermark.style.cssText = `
				position: absolute;
				bottom: 15px;
				right: 15px;
				display: flex;
				align-items: center;
				gap: 8px;
				color: ${watermarkColor};
				font-size: 14px;
				font-family: system-ui;
				pointer-events: none;
				z-index: 9999;
			`;

			const logo = document.createElement('img');
			logo.src = '/logos/scanopy-logo.png';
			logo.style.cssText = `
				height: 18px;
				width: auto;
			`;

			const text = document.createElement('span');
			text.textContent = topology_createdUsing();

			watermark.appendChild(logo);
			watermark.appendChild(text);
			flowElement.appendChild(watermark);

			await new Promise<void>((resolve) => {
				if (logo.complete) {
					resolve();
				} else {
					logo.onload = () => resolve();
					logo.onerror = () => resolve();
				}
			});
		}

		await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));

		try {
			await callback({ flowElement, imageWidth, imageHeight });
		} finally {
			isExporting.set(false);
			header.remove();
			watermark.remove();
			flowElement.classList.remove('hide-for-export');
			flowElement.style.width = originalWidth;
			flowElement.style.height = originalHeight;
			if (needsThemeSwitch) {
				document.documentElement.classList.toggle('dark', currentTheme === 'dark');
				document.documentElement.style.colorScheme = currentTheme;
			}
			setTimeout(() => setViewport(originalViewport, { duration: 0 }), 50);
		}
	}

	async function captureImage(format: 'png' | 'svg') {
		await withExportCapture(async ({ flowElement, imageWidth, imageHeight }) => {
			const options = {
				width: imageWidth,
				height: imageHeight,
				pixelRatio: effectivePixelRatio(),
				imagePlaceholder: LOGO_FALLBACK,
				onImageErrorHandler: () => {}
			};
			const dataUrl =
				format === 'svg' ? await toSvg(flowElement, options) : await toPng(flowElement, options);

			const link = document.createElement('a');
			link.download = buildExportFilename(format);
			link.href = dataUrl;
			link.click();
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format });
		});
	}

	function triggerDownload(blob: Blob, filename: string) {
		const url = URL.createObjectURL(blob);
		const link = document.createElement('a');
		link.download = filename;
		link.href = url;
		link.click();
		URL.revokeObjectURL(url);
	}

	async function handlePdfExport() {
		await withExportCapture(async ({ flowElement, imageWidth, imageHeight }) => {
			const options = {
				width: imageWidth,
				height: imageHeight,
				pixelRatio: effectivePixelRatio(),
				imagePlaceholder: LOGO_FALLBACK,
				onImageErrorHandler: () => {}
			};
			const pngDataUrl = await toPng(flowElement, options);

			// Convert PNG data URL to JPEG via canvas for smaller PDF size
			const img = new window.Image();
			img.src = pngDataUrl;
			await new Promise<void>((resolve) => {
				img.onload = () => resolve();
			});

			const canvas = document.createElement('canvas');
			canvas.width = img.width;
			canvas.height = img.height;
			const ctx = canvas.getContext('2d')!;
			ctx.fillStyle = exportTheme === 'dark' ? '#1a1a2e' : '#ffffff';
			ctx.fillRect(0, 0, canvas.width, canvas.height);
			ctx.drawImage(img, 0, 0);

			const jpegDataUrl = canvas.toDataURL('image/jpeg', 0.92);
			const jpegBase64 = jpegDataUrl.split(',')[1];
			const jpegBytes = Uint8Array.from(atob(jpegBase64), (c) => c.charCodeAt(0));

			const pdfBytes = buildPdf(jpegBytes, canvas.width, canvas.height);

			const pdfBuffer = new ArrayBuffer(pdfBytes.byteLength);
			new Uint8Array(pdfBuffer).set(pdfBytes);
			triggerDownload(
				new Blob([pdfBuffer], { type: 'application/pdf' }),
				buildExportFilename('pdf')
			);
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'pdf' });
		});
	}

	function buildPdf(jpegBytes: Uint8Array, imgWidth: number, imgHeight: number): Uint8Array {
		const isDark = exportTheme === 'dark';
		const pageWidth = 842; // A4 landscape points
		const pageHeight = 595;
		const margin = 40;
		const availableWidth = pageWidth - margin * 2;
		const availableHeight = pageHeight - margin * 2;
		const scale = Math.min(availableWidth / imgWidth, availableHeight / imgHeight);
		const drawWidth = Math.round(imgWidth * scale);
		const drawHeight = Math.round(imgHeight * scale);
		const drawX = margin + (availableWidth - drawWidth) / 2;
		const drawY = margin;

		const encoder = new TextEncoder();
		const parts: Uint8Array[] = [];
		const offsets: number[] = [];
		let pos = 0;

		function write(str: string) {
			const bytes = encoder.encode(str);
			parts.push(bytes);
			pos += bytes.length;
		}

		function writeBinary(bytes: Uint8Array) {
			parts.push(bytes);
			pos += bytes.length;
		}

		function recordOffset() {
			offsets.push(pos);
		}

		write('%PDF-1.4\n%\xFF\xFF\xFF\xFF\n');

		recordOffset();
		write('1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n');

		recordOffset();
		write(`2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n`);

		recordOffset();
		write(
			`3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${pageWidth} ${pageHeight}] /Contents 4 0 R /Resources << /XObject << /Img 5 0 R >> /Font << /F1 6 0 R >> >> >>\nendobj\n`
		);

		// Background fill
		const bgR = isDark ? 0.102 : 1;
		const bgG = isDark ? 0.102 : 1;
		const bgB = isDark ? 0.18 : 1;
		const bgRect = `${bgR} ${bgG} ${bgB} rg 0 0 ${pageWidth} ${pageHeight} re f\n`;
		const imgCmd = `q ${drawWidth} 0 0 ${drawHeight} ${drawX} ${drawY} cm /Img Do Q\n`;
		const contentStream = bgRect + imgCmd;

		recordOffset();
		write(
			`4 0 obj\n<< /Length ${contentStream.length} >>\nstream\n${contentStream}endstream\nendobj\n`
		);

		recordOffset();
		write(
			`5 0 obj\n<< /Type /XObject /Subtype /Image /Width ${imgWidth} /Height ${imgHeight} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length ${jpegBytes.length} >>\nstream\n`
		);
		writeBinary(jpegBytes);
		write('\nendstream\nendobj\n');

		recordOffset();
		write('6 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n');

		const xrefPos = pos;
		write('xref\n');
		write(`0 ${offsets.length + 1}\n`);
		write('0000000000 65535 f \n');
		for (const offset of offsets) {
			write(`${String(offset).padStart(10, '0')} 00000 n \n`);
		}

		write('trailer\n');
		write(`<< /Size ${offsets.length + 1} /Root 1 0 R >>\n`);
		write('startxref\n');
		write(`${xrefPos}\n`);
		write('%%EOF\n');

		const totalLength = parts.reduce((sum, p) => sum + p.length, 0);
		const result = new Uint8Array(totalLength);
		let offset = 0;
		for (const part of parts) {
			result.set(part, offset);
			offset += part.length;
		}
		return result;
	}

	async function handleHtmlExport() {
		await withExportCapture(async ({ flowElement, imageWidth, imageHeight }) => {
			const options = {
				width: imageWidth,
				height: imageHeight,
				pixelRatio: effectivePixelRatio(),
				imagePlaceholder: LOGO_FALLBACK,
				onImageErrorHandler: () => {}
			};
			const pngDataUrl = await toPng(flowElement, options);

			const bgColor = exportTheme === 'dark' ? '#1a1a2e' : '#ffffff';
			const textColor = exportTheme === 'dark' ? '#e0e0e0' : '#333333';
			const watermarkHtml = !hideCreatedWith
				? `<p style="margin-top:20px;color:${exportTheme === 'dark' ? 'rgba(255,255,255,0.5)' : 'rgba(0,0,0,0.3)'};font-size:13px;">${topology_createdUsing()}</p>`
				: '';

			const styleTag = 'style';
			const css = `body{margin:0;padding:40px;background:${bgColor};color:${textColor};font-family:system-ui,-apple-system,sans-serif;text-align:center;}img{max-width:100%;height:auto;border-radius:8px;}`;
			const html = [
				'<!DOCTYPE html>',
				'<html lang="en">',
				'<head>',
				'<meta charset="UTF-8">',
				'<meta name="viewport" content="width=device-width, initial-scale=1.0">',
				`<title>${escapeXml(exportHeader(new Date()).title)}</title>`,
				`<${styleTag}>${css}</${styleTag}>`,
				'</head>',
				'<body>',
				`<img src="${pngDataUrl}" alt="Topology" />`,
				watermarkHtml,
				'</body>',
				'</html>'
			].join('\n');

			triggerDownload(new Blob([html], { type: 'text/html' }), buildExportFilename('html'));
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'html' });
		});
	}

	async function handleDiagramExport(format: 'drawio' | 'vsdx') {
		await withExportCapture(
			async () => {
				try {
					const { buildDiagramExport } = await import('../diagram-export');
					const blob = await buildDiagramExport({
						format,
						name: exportHeader(new Date()).title,
						nodes: getNodes(),
						edges: getEdges(),
						getInternalNode,
						topology,
						hiddenEdgeTypes: $hiddenEdgeTypes,
						theme: exportTheme,
						header: includeHeader ? exportHeader(new Date()) : null
					});
					triggerDownload(blob, buildExportFilename(format));
					pushSuccess(topology_exportComplete());
					trackEvent('topology_exported', { format });
				} catch {
					pushError(topology_exportFailed());
				}
			},
			{ decorate: false }
		);
	}

	async function handleMermaidExport() {
		close();
		try {
			await downloadTopologyExport(topologyId, 'mermaid', $activeView);
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'mermaid' });
		} catch {
			pushError(topology_exportFailed());
		}
	}

	async function handleConfluenceExport() {
		close();
		try {
			await downloadTopologyExport(topologyId, 'confluence', $activeView);
			pushSuccess(topology_exportComplete());
			trackEvent('topology_exported', { format: 'confluence' });
		} catch {
			pushError(topology_exportFailed());
		}
	}

	function handleUpgrade(feature: UpgradeFeature) {
		triggerUpgrade({
			feature,
			source: 'export_modal',
			surface: 'export_modal',
			beforeModal: () => {
				close();
			}
		});
	}

	let actions = $derived<ModalAction[]>([
		{
			role: 'primary',
			label: common_export(),
			onClick: handleExport,
			disabled: selectedFormatGated
		}
	]);
</script>

<GenericModal title={topology_export()} {name} {isOpen} onClose={close} size="sm" {actions}>
	<div class="p-6">
		<div class="space-y-4">
			<RichSelect
				label=""
				selectedValue={selectedFormat}
				options={formatOptions}
				onSelect={handleFormatSelect}
				onDisabledClick={handleDisabledFormatClick}
				displayComponent={SimpleOptionDisplay}
			/>

			{#if selectedFormatDef?.supportsTheme}
				<div class="flex items-center justify-between">
					<span class="text-sm font-medium text-secondary">{topology_exportTheme()}</span>
					<div class="flex gap-1">
						<button
							class="btn-secondary gap-1 px-2.5! py-1! text-xs! {exportTheme === 'light'
								? 'list-item-selected'
								: ''}"
							onclick={() => (exportTheme = 'light')}
						>
							<Sun size={12} />
							{common_light()}
						</button>
						<button
							class="btn-secondary gap-1 px-2.5! py-1! text-xs! {exportTheme === 'dark'
								? 'list-item-selected'
								: ''}"
							onclick={() => (exportTheme = 'dark')}
						>
							<Moon size={12} />
							{common_dark()}
						</button>
					</div>
				</div>
			{/if}

			{#if selectedFormatDef?.supportsHeader}
				<!-- One block: the checkbox and the header it adds, so the preview reads as its result. -->
				<label class="card-static card flex cursor-pointer items-start gap-2 px-3 py-2">
					<input
						type="checkbox"
						class="checkbox-card mt-0.5 h-4 w-4"
						bind:checked={includeHeader}
					/>
					<span class="min-w-0 space-y-1">
						<span class="block text-sm text-secondary">{topology_exportIncludeHeader()}</span>
						<span class="block text-xs text-tertiary">
							<span class="block truncate font-medium">{headerPreview.title}</span>
							<span class="block">{headerPreview.subtitle}</span>
						</span>
					</span>
				</label>
			{/if}
		</div>
	</div>
</GenericModal>
