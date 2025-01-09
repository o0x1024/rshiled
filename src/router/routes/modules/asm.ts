
const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const ASM: AppRouteRecordRaw = {
	path: '/',
	component: HOME,
	redirect:'/asm/scan-object',
	meta: {
		locale: 'menu.profile',
		requiresAuth: true,
		icon: 'icon-file',
		order: 4,
	},
	children: [
		{
			path: 'asm',
			name: 'asm',
			redirect:'/asm/scan-object',
			component: () => import('@/views/asm/index.vue'),
			children:[
				{
					path: 'dashboard',
					name: 'asm-dashboard',
					component: () => import('@/views/asm/components/dashboard.vue'),
				},
				{
					path: 'scan-object',
					name: 'asm-scan-object',
					component: () => import('@/views/asm/components/enterprise.vue'),
				},				
				{
					path: 'root-domain',
					name: 'asm-root-domain',
					component: () => import('@/views/asm/components/root-domain.vue'),
				},				
				{
					path: 'domain',
					name: 'asm-domain',
					component: () => import('@/views/asm/components/domain.vue'),
				},				
				{
					path: 'ip',
					name: 'asm-ip',
					component: () => import('@/views/asm/components/ip.vue'),
				},				
				{
					path: 'port',
					name: 'asm-port',
					component: () => import('@/views/asm/components/port.vue'),
				},				
				{
					path: 'website',
					name: 'asm-website',
					component: () => import('@/views/asm/components/website.vue'),
				},				
				{
					path: 'web-component',
					name: 'asm-web-component',
					component: () => import('@/views/asm/components/web-component.vue'),
				},				
				{
					path: 'risk',
					name: 'asm-risk',
					component: () => import('@/views/asm/components/risk.vue'),
				},				
			]
		}
	]
};

export default ASM;
