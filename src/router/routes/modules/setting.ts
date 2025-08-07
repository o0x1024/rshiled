const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const SETTING: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.setting',
		order: 6,
	},
	children: [
		{
			path: 'setting',
			name: 'setting',
			redirect:'/setting/scan',
			component: () => import('@/views/setting/index.vue'),
			meta:{
				locale:'menu.setting'
			},
			children:[
				{
					path:'asm',
					name:'setting-asm',
					component: () => import('@/views/setting/components/asm.vue'),
					meta:{
						locale:'setting.asm'
					}
				},
				{
					path:'scan',
					name:'setting-scan',
					component: () => import('@/views/setting//components/scan.vue'),
					meta:{
						locale:'setting.scan'
					}
				}
			]
		}
	]
};

export default SETTING;
