import { AppRouteRecordRaw } from '../types';

const LOG_VIEWER: AppRouteRecordRaw = {
  path: '/log-viewer',
  name: 'log-viewer',
  component: () => import('@/views/log-viewer/index.vue'),
  meta: {
    hideInMenu: true,
    noAffix: true,
  },
};

export default LOG_VIEWER; 