import '../app.css';

// 该应用是 adapter-static SPA(fallback: index.html),数据全靠运行时 API。
// 关闭 SSR:dev 与生产一致走客户端渲染,避免 dev SSR 阶段跑鉴权/查询卡住。
export const ssr = false;

