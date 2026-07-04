// n9e 嵌入模式开关。
// app.html 的 inline 脚本从 iframe 的 ?embed=1(+localStorage)注入 window.__SCANOPY_EMBED__。
// 嵌入 n9e 时隐藏 scanopy 自带的 SaaS 壳(billing/banner/Upgrade/PlanUsage/品牌等)。
// 原生部署无该参数 → false → 行为完全不变。
export const isEmbed: boolean =
	typeof window !== 'undefined' &&
	!!(window as unknown as { __SCANOPY_EMBED__?: boolean }).__SCANOPY_EMBED__;
