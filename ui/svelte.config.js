import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		adapter: adapter({
			fallback: 'index.html'
		}),
		// n9e 嵌入:子路径托管(资源/路由前缀)。构建期 Node env,原生构建不设 → 空。
		// 必须以 / 开头且不以 / 结尾,如 /topo-studio。
		paths: {
			base: process.env.SCANOPY_BASE_PATH || ''
		}
	}
};

export default config;
