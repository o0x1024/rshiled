
const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const SETTING: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		locale: 'menu.setting',
		order: 1,
	},
	children: [
		{
			path: 'setting',
			name: 'setting',
			component: () => import('@/views/setting/index.vue'),
			meta:{
				locale:'menu.setting'
			}
		}
	]
};

export default SETTING;
