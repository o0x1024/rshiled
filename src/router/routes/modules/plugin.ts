
const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const PLUGIN: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	meta: {
		order: 1,
	},
	children: [
		{
			path: 'plugin',
			name: 'plugin',
			component: () => import('@/views/plugin/index.vue'),
			meta:{
				locale:'menu.plugin'
			}
		}
	]
};

export default PLUGIN;
