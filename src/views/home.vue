<template>
	<a-layout class="layout">
		<div class="layout-navbar">
			<NavBar />
		</div>
		<a-layout>
			<a-layout>
				<a-layout :style="paddingStyle">
					<a-layout-content>
						<RouterView v-slot="{ Component }">
							<keep-alive :include="cachedViews">
								<component :is="Component" />
							</keep-alive>
						</RouterView>
					</a-layout-content>
				</a-layout>
			</a-layout>
		</a-layout>
	</a-layout>
</template>

<script setup lang="ts">
import NavBar from '@/components/navbar/index.vue';
import { useAppStore } from '@/store';
import { computed} from 'vue';
import { useRouter } from 'vue-router';

const router = useRouter()


const navbarHeight = `60px`;
const appStore = useAppStore();
const navbar = computed(() => appStore.navbar);
const menuWidth = computed(() => {
	return appStore.menuCollapse ? 48 : appStore.menuWidth;
});


// 根据路由meta.keepAlive属性判断是否需要缓存
const cachedViews = computed(() => {
  const cacheList: string[] = [];
  const routes = router.getRoutes();
  routes.forEach(route => {
    if (route.meta && route.meta.keepAlive && route.name) {
      cacheList.push(route.name.toString());
      if (route.meta.componentName) {
        cacheList.push(route.meta.componentName.toString());
      }
    }
  });
  console.log('Home cached views:', cacheList);
  return cacheList;
});


const paddingStyle = computed(() => {
	const paddingLeft = { paddingLeft: `${menuWidth.value}px` }
	const paddingTop = navbar.value ? { paddingTop: navbarHeight } : {};
	return { ...paddingLeft, ...paddingTop };
});

</script>

<style scoped lang="less">
@nav-size-height: 60px;


.layout-navbar {
	position: fixed;
	top: 0;
	left: 0;
	z-index: 100;
	width: 100%;
	height: @nav-size-height;
}
</style>