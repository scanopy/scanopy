// n9e 嵌入:补齐 scanopy 里未走 paraglide 的硬编码英文文案(Home 区块标题、拓扑详情标签等)。
// locale=zh 时查下表返回中文,否则原样返回英文。原生(非 zh)行为不变。
// 注:这是针对"漏网英文"的轻量补丁;若将来要支持更多语言,应迁移到 paraglide 消息。
import { getLocale } from '$lib/paraglide/runtime';

const ZH: Record<string, string> = {
	// Home
	'Active Discoveries': '进行中的发现',
	'Recent Discoveries': '最近发现',
	Daemons: '采集器',
	Networks: '网络',
	// 拓扑详情面板
	Source: '源',
	Target: '目标',
	'Hypervisor Host': '虚拟化宿主机',
	'Docker Host': 'Docker 宿主机',
	'Docker Service': 'Docker 服务',
	Dependency: '依赖',
	Services: '服务',
	'Edge data not available': '无连线数据',
	'Unable to display edge details': '无法显示连线详情',
	'Unable to display node details': '无法显示节点详情'
};

export function et(en: string): string {
	return getLocale() === 'zh' ? (ZH[en] ?? en) : en;
}

// "No <label>" 拼接的本地化:中文 → "无<label>",英文 → "No <label>"。
// 修 GenericCard/ListManager 里 `No ${label}` 导致的 "No 凭据" 混排。
export function noneOf(label: string): string {
	return getLocale() === 'zh' ? `无${label}` : `No ${label.toLowerCase()}`;
}
