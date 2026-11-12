import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { readFileSync } from 'node:fs';
import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import license, { type Dependency } from 'rollup-plugin-license';
import { defineConfig } from 'vitest/config';
import pkg from './package.json' with { type: 'json' };

// The licences the UI bundle accepts from its npm packages. A package under any other licence
// (GPL, LGPL, AGPL...) fails `npm run build`, and so ui-ci. backend/about.toml holds the same
// list for crates. EPL-2.0 is for elkjs (EPL-2.0 OR GPL-3.0-or-later), used unmodified.
const ACCEPTED_LICENSES =
	'(MIT OR MIT-0 OR ISC OR Apache-2.0 OR BSD-2-Clause OR BSD-3-Clause OR 0BSD OR Unlicense OR CC0-1.0 OR BlueOak-1.0.0 OR Python-2.0 OR MPL-2.0 OR EPL-2.0)';

const BACKEND_NOTICES = new URL('../backend/THIRD-PARTY-NOTICES.txt', import.meta.url);

function repositoryUrl(dependency: Dependency): string | null {
	const repository = dependency.repository;
	if (!repository) return dependency.homepage;
	return typeof repository === 'string' ? repository : repository.url;
}

// /third-party-licenses.txt: the server's and daemon's notices (backend/THIRD-PARTY-NOTICES.txt,
// from `make third-party-notices`), then every npm package bundled into the UI.
function thirdPartyLicenses(dependencies: Dependency[]): string {
	const sections = dependencies
		.slice()
		.sort((a, b) => (a.name ?? '').localeCompare(b.name ?? ''))
		.map((dependency) => {
			const repository = repositoryUrl(dependency);
			const heading = `${dependency.name} ${dependency.version} (${dependency.license ?? 'no licence declared'})`;
			return [
				'-'.repeat(80),
				heading,
				...(repository ? [repository] : []),
				'-'.repeat(80),
				'',
				dependency.licenseText ?? '',
				...(dependency.noticeText ? ['', dependency.noticeText] : [])
			].join('\n');
		});
	return [
		readFileSync(BACKEND_NOTICES, 'utf8'),
		'',
		'='.repeat(80),
		'NPM PACKAGES (web UI)',
		'='.repeat(80),
		'',
		sections.join('\n\n'),
		''
	].join('\n');
}

export default defineConfig({
	test: {
		include: ['src/tests/**/*.test.ts']
	},
	plugins: [
		sveltekit({
			preprocess: vitePreprocess(),
			adapter: adapter({ fallback: 'index.html' }),
			// SvelteKit 3 polls for a new build every hour by default. Nothing reads `updated`,
			// so keep polling off, as it was on SvelteKit 2.
			version: { pollInterval: 0 }
		}),
		paraglideVitePlugin({ project: './project.inlang', outdir: './src/lib/paraglide' }),
		{
			...license({
				thirdParty: {
					includePrivate: false,
					multipleVersions: true,
					allow: { test: ACCEPTED_LICENSES, failOnUnlicensed: true, failOnViolation: true },
					// adapter-static copies the client output into build/, so the server serves it.
					output: {
						file: '.svelte-kit/output/client/third-party-licenses.txt',
						template: thirdPartyLicenses
					}
				}
			}),
			// The client bundle is what ships to browsers; the SSR build only prerenders.
			apply: (_, env) => env.command === 'build' && !env.isSsrBuild
		},
		{
			// The dev server has no build output, so serve the backend notices on their own at the
			// same path. The npm section exists only in a production build.
			name: 'third-party-licenses-dev',
			apply: 'serve',
			configureServer(server) {
				server.middlewares.use('/third-party-licenses.txt', (_req, res) => {
					res.setHeader('Content-Type', 'text/plain; charset=utf-8');
					res.end(readFileSync(BACKEND_NOTICES, 'utf8'));
				});
			}
		}
	],
	define: {
		__APP_VERSION__: JSON.stringify(pkg.version)
	},
	server: {
		host: '0.0.0.0',
		allowedHosts: ['scanopy-dev.local'],
		port: 5173,
		proxy: {
			'/api': {
				target: 'http://localhost:60072',
				changeOrigin: true
			}
		}
	},

	// The server build externalizes bare package imports, which leaves `@import 'tailwindcss'` in
	// app.css unresolved. Bundling it lets the CSS resolver find the package's stylesheet.
	ssr: {
		noExternal: ['tailwindcss']
	},

	build: {
		outDir: 'build'
	}
});
