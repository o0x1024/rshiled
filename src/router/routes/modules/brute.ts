const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const BRUTE: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.brute',
		order: 4,
		keepAlive: true,
		componentName: 'brute-home'
	},
	children: [
		{
			path: 'brute',
			name: 'brute',
			component: () => import('@/views/brute/index.vue'),
			meta: {
				keepAlive: true,
				componentName: 'brute-home'
			},
			// children: [
			// 	{
			// 		path: 'dir',
			// 		name: 'scan-dir',
			// 		component: () => import('@/views/scan/components/dir.vue'),
			// 		meta:{
			// 			locale:'vuln.exploit'
			// 		}
			// 	},


			// ]
		}
	]
};

export default BRUTE;
