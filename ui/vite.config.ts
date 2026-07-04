import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vitest/config';
import pkg from './package.json';

export default defineConfig({
	test: {
		include: ['src/tests/**/*.test.ts']
	},
	plugins: [
		sveltekit(),
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide'
		})
	],
	define: {
		__APP_VERSION__: JSON.stringify(pkg.version)
	},
	server: {
		host: '0.0.0.0',
		allowedHosts: ['scanopy-dev.local'],
		port: 5173,
		proxy: {
			// dev 时 /api 代理到后端。默认本地 scanopy(原生开发不变)。
			// n9e 嵌入调试:设 SCANOPY_DEV_API_TARGET=http://localhost:18080(本地 topo-studio,
			// 注入服务会话)+ SCANOPY_DEV_API_REWRITE=/scanopy-api → 走会话代理到远程 scanopy。
			'/api': {
				target: process.env.SCANOPY_DEV_API_TARGET || 'http://localhost:60072',
				changeOrigin: true,
				...(process.env.SCANOPY_DEV_API_REWRITE
					? { rewrite: (p: string) => process.env.SCANOPY_DEV_API_REWRITE + p }
					: {})
			}
		}
	},

	build: {
		outDir: 'build'
	}
});
