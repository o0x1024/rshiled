import { createRouter, createWebHistory } from 'vue-router'
import { appRoutes } from './routes';


const router = createRouter({
  history: createWebHistory(),
  routes: [
    ...appRoutes,
  ],
})

// 添加导航守卫，记录路由跳转信息以便调试
router.beforeEach((to, from, next) => {
  console.log('Route navigation:', { 
    from: from.fullPath, 
    to: to.fullPath, 
    fromName: from.name, 
    toName: to.name
  });
  next();
});


export default router;