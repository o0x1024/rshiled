
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
			redirect:'/plugin/list',
			component: () => import('@/views/plugin/index.vue'),
			meta:{
				locale:'menu.plugin'
			},
			children: [
				{
					path: 'list',
					name: 'plugin-list',
					component: () => import('@/views/plugin/components/list.vue'),
					meta:{
						locale:'menu.plugin.list'
					}
				},
				// {
				// 	path: 'add',
				// 	name: 'plugin-add',
				// 	component: () => import('@/views/plugin/add.vue'),
				// 	meta:{
				// 		locale:'menu.plugin.add'
				// 	}
				// },
				// {
				// 	path: 'edit',
				// 	name: 'pluginEdit',
				// 	component: () => import('@/views/plugin/edit.vue'),
				// 	meta:{
				// 		locale:'menu.plugin.edit'
				// 	}
				// }
			]

		}
	]
};

export default PLUGIN;
