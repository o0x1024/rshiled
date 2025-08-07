const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const ASM: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	redirect:'/asm',
	meta: {
		locale: 'menu.asm',
		requiresAuth: true,
		icon: 'icon-file',
		order: 1,
		keepAlive: true,
		componentName: 'asm-home'
	},
	children: [
		{
			path: 'asm',
			name: 'asm',
			redirect:'/asm/scan-object',
			component: () => import('@/views/asm/index.vue'),
			meta: {
				locale: 'menu.asm',
				componentName: 'asm-home'
			},
			children:[
				{
					path: 'dashboard',
					name: 'asm-dashboard',
					component: () => import('@/views/asm/components/dashboard.vue'),
					meta: {
						componentName: 'asm-dashboard'
					}
				},
				{
					path: 'scan-object',
					name: 'asm-scan-object',
					component: () => import('@/views/asm/components/scantask.vue'),
					meta: {
						componentName: 'asm-scan-object'
					}
				},				
				{
					path: 'root-domain',
					name: 'asm-root-domain',
					component: () => import('@/views/asm/components/root-domain.vue'),
					meta: {
					}
				},				
				{
					path: 'domain',
					name: 'asm-domain',
					component: () => import('@/views/asm/components/domain.vue'),
					meta: {
					}
				},				
				{
					path: 'ip',
					name: 'asm-ip',
					component: () => import('@/views/asm/components/ip.vue'),
					meta: {
					}
				},				
				{
					path: 'port',
					name: 'asm-port',
					component: () => import('@/views/asm/components/port.vue'),
					meta: {
					}
				},				
				{
					path: 'website',
					name: 'asm-website',
					component: () => import('@/views/asm/components/website.vue'),
					meta: {
					}
				},				
				{
					path: 'web-component',
					name: 'asm-web-component',
					component: () => import('@/views/asm/components/web-component.vue'),
					meta: {
					}
				},				
				{
					path: 'risk',
					name: 'asm-risk',
					component: () => import('@/views/asm/components/risk.vue'),
					meta: {
						// keepAlive: true
					}
				},		
				{
					path: 'api',
					name: 'asm-api',
					component: () => import('@/views/asm/components/api.vue'),
					meta: {
						componentName: 'asm-api'
					}
				},	
				{
					path: 'plugin',
					name: 'asm-plugin',
					component: () => import('@/views/asm/components/plugin.vue'),
					meta:{
						locale:'menu.plugin.list',
						keepAlive: true,
						componentName: 'asm-plugin'
					}
				},
				{
					path: 'visualization',
					name: 'asm-visualization',
					component: () => import('@/views/asm/components/visualization/index.vue'),
					redirect: '/asm/visualization/asset-graph',
					children: [
						{
							path: 'asset-graph',
							name: 'asm-visualization-asset-graph',
							component: () => import('@/views/asm/components/visualization/asset-graph.vue'),
							meta: {
								locale: 'asm.visualization.asset_graph',
								keepAlive: true,
								componentName: 'asm-visualization-asset-graph'
							}
						},
						{
							path: 'risk-heatmap',
							name: 'asm-visualization-risk-heatmap',
							component: () => import('@/views/asm/components/visualization/risk-heatmap.vue'),
							meta: {
								locale: 'asm.visualization.risk_heatmap',
								keepAlive: true,
								componentName: 'asm-visualization-risk-heatmap'
							}
						},
						{
							path: 'compliance-report',
							name: 'asm-visualization-compliance-report',
							component: () => import('@/views/asm/components/visualization/compliance-report.vue'),
							meta: {
								locale: 'asm.visualization.compliance_report',
								keepAlive: true,
								componentName: 'asm-visualization-compliance-report'
							}
						}
					]
				},
			]
		}
	]
};

export default ASM;
