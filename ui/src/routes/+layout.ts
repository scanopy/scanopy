import '../app.css';
import '../n9e-bridge.css';
import { overwriteGetLocale } from '$lib/paraglide/runtime';

// 该应用是 adapter-static SPA(fallback: index.html),数据全靠运行时 API。
// 关闭 SSR:dev 与生产一致走客户端渲染,避免 dev SSR 阶段跑鉴权/查询卡住。
export const ssr = false;

// n9e 语言联动:确定性地从 iframe ?locale= 或 PARAGLIDE_LOCALE cookie 定语言,
// 绕过 Paraglide cookie strategy 的时序不确定性。原生部署无参数无 cookie → en。
if (typeof window !== 'undefined') {
	const p = new URLSearchParams(window.location.search).get('locale');
	const m = document.cookie.match(/(?:^|; )PARAGLIDE_LOCALE=(zh|en)/);
	const loc = (p === 'zh' || p === 'en' ? p : m ? m[1] : 'en') as 'en' | 'zh';
	overwriteGetLocale(() => loc);
}

