
const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const BRUTE: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.scan',
		order: 1,
	},
	children: [
		{
			path: 'brute',
			name: 'brute',
			component: () => import('@/views/brute/index.vue'),
			children: [
				{
					path: 'dir',
					name: 'scan-dir',
					component: () => import('@/views/scan/components/dir.vue'),
					meta:{
						locale:'vuln.exploit'
					}
				},


			]
		}
	]
};

export default BRUTE;
