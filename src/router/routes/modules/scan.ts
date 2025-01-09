
const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const SCAN: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.scan',
		order: 1,
	},
	children: [
		{
			path: 'scan',
			name: 'scan',
			component: () => import('@/views/scan/index.vue'),
			children: [
				{
					path: 'dir',
					name: 'scan-dir',
					component: () => import('@/views/scan/components/dir.vue'),
					meta:{
						locale:'vuln.exploit'
					}
				},
				{
					path: 'port',
					name: 'scan-port',
					component: () => import('@/views/scan/components/port.vue'),
					meta:{
						locale:'vuln.exploit'
					}
				},

			]
		}
	]
};

export default SCAN;
