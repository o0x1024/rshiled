
const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const VULN: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.vuln',
		order: 1,
	},
	children: [
		{
			path: 'vuln',
			name: 'vuln',
			redirect:'/vuln/exploit',
			component: () => import('@/views/vuln/index.vue'),
			children: [
				{
					path: 'exploit',
					name: 'vuln-exploit',
					component: () => import('@/views/vuln/components/exploit.vue'),
					meta:{
						locale:'vuln.exploit'
					}
				},
				{
					path: 'role',
					name: 'plugin-role',
					component: () => import('@/views/vuln/components/plugin-role.vue'),
					meta:{
						locale:'vuln.plugin-role'
					}
				},
				{
					path: 'plugin',
					name: 'vuln-plugin',
					component: () => import('@/views/vuln/components/plugin.vue'),
					meta:{
						locale:'vuln.plugin'
					}
				}
			]
		},
		
	]
};

export default VULN;
