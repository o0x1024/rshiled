const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const SCAN: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.scan',
		order: 3,
		keepAlive: true,
		componentName: 'scan-home'
	},
	children: [
		{
			path: 'scan',
			name: 'scan',
			redirect:'/scan/active',
			component: () => import('@/views/scan/index.vue'),
			meta: {
				keepAlive: true,
				componentName: 'scan-home'
			},
			children: [
				{
					path: 'active',
					name: 'scan-active',
					component: () => import('@/views/scan/components/active.vue'),
					meta:{
						locale:'scan.active'
					}
				},
				{
					path: 'passive',
					name: 'scan-passive',
					component: () => import('@/views/scan/components/passive.vue'),
					meta:{
						locale:'scan.passive'
					}
				},
				{
					path: 'plugins',
					name: 'scan-plugins',
					component: () => import('@/views/scan/components/plugins.vue'),
					meta:{
						locale:'scan.plugins'
					}
				}
			]
		}
	]
};

export default SCAN;
