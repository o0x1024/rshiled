const HOME = () => import('@/views/home.vue')
import { AppRouteRecordRaw } from '../types';

const PROXY: AppRouteRecordRaw = {
  path: '/',
  component: HOME,
  redirect: '/proxy',
  meta: {
    locale: 'menu.proxy',
    requiresAuth: true,
    icon: 'icon-swap',
    order: 4,
    keepAlive: true,
    componentName: 'proxy-home'
  },
  children: [
    {
      path: 'proxy',
      name: 'proxy',
      redirect: '/proxy/intercept',
      component: () => import('@/views/proxy/index.vue'),
      meta: {
        locale: 'menu.proxy',
        componentName: 'proxy-home'
      },
      children: [
        {
          path: 'intercept',
          name: 'proxy-intercept',
          component: () => import('@/views/proxy/components/intercept.vue'),
          meta: {
            locale: 'proxy.intercept',
            componentName: 'proxy-intercept'
          }
        },
        {
          path: 'history',
          name: 'proxy-history',
          component: () => import('@/views/proxy/components/history.vue'),
          meta: {
            locale: 'proxy.history',
            componentName: 'proxy-history'
          }
        },
        {
          path: 'settings',
          name: 'proxy-settings',
          component: () => import('@/views/proxy/components/settings.vue'),
          meta: {
            locale: 'proxy.settings',
            componentName: 'proxy-settings'
          }
        }
      ]
    }
  ]
};

export default PROXY; 