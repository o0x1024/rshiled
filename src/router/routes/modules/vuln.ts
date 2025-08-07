const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const VULN: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.vuln',
		order: 2,
		keepAlive: true,
		componentName: 'vuln-home'
	},
	children: [
		{
			path: 'vuln',
			name: 'vuln',
			redirect:'/vuln/exploit',
			component: () => import('@/views/vuln/index.vue'),
			meta: {
				locale: 'vuln.title',
				keepAlive: true,
				componentName: 'vuln-home'
			},
			children: [
				{
					path: 'exploit',
					name: 'vuln-exploit',
					component: () => import('@/views/vuln/components/exploit.vue'),
					meta:{
						locale:'vuln.menu.exploit',
						keepAlive: true
					}
				},
				{
					path: 'plugin',
					name: 'vuln-plugin',
					component: () => import('@/views/vuln/components/plugin.vue'),
					meta:{
						locale:'vuln.menu.plugin',
						keepAlive: true
					}
				}
			]
		},
		
	]
};

export default VULN;
