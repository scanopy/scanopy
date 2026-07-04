/** @type {import('tailwindcss').Config} */

export default {
	darkMode: 'class',
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		extend: {
			colors: {
				// n9e 嵌入:Tailwind blue 色阶整体重映射到 n9e 主色 #0078c2 系,
				// 让所有 bg-blue-*/text-blue-*/border-blue-*(按钮/链接/强调)对齐 n9e。
				blue: {
					50: '#e6f4fb',
					100: '#cce9f6',
					200: '#99d3ed',
					300: '#66bce4',
					400: '#2699e0',
					500: '#0078c2',
					600: '#006cae',
					700: '#005f99',
					800: '#004876',
					900: '#003558',
					950: '#002238'
				},
				// primary(原 sky)同步到 n9e 蓝,避免两种蓝并存
				primary: {
					50: '#e6f4fb',
					100: '#cce9f6',
					500: '#0078c2',
					600: '#006cae',
					700: '#005f99',
					900: '#003558'
				},
				gray: {
					800: '#1f2937',
					900: '#111827'
				},
				success: '#10b981',
				warning: '#f59e0b',
				error: '#ef4444'
			},
			fontFamily: {
				mono: ['Monaco', 'Consolas', 'Liberation Mono', 'monospace']
			}
		}
	},
	plugins: [require('@tailwindcss/forms')]
};
