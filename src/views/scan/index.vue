<template>
	<a-layout class="layout">
		<a-layout>
			<a-layout-sider class="layout-sider" @collapse="setCollapsed"  :width="menuWidth" :style="{ paddingTop: '60px' }">
				<div class="menu-wrapper">
					<a-menu :style="{ height: 'calc(100% - 0px)' }"  @collapse="setCollapsed" @menu-item-click="onMenuClick" mode="pop"
						showCollapseButton :selected-keys="selectedKeys">
						<a-menu-item key="1">
							<template #icon><icon-dashboard></icon-dashboard></template>
							{{ $t('scan.active_scan') }}
						</a-menu-item>
						<a-menu-item key="2">
							<template #icon><icon-book /></template>
							{{ $t('scan.passive_scan') }}
						</a-menu-item>
						<a-divider />
						<a-menu-item key="3">
							<template #icon><icon-book /></template>
							{{ $t('scan.plugins') }}
						</a-menu-item>
					</a-menu>
				</div>
			</a-layout-sider>
			<a-layout-content class="layout-content" :style="paddingStyle">
				<RouterView v-slot="{ Component }">
					<keep-alive :include="cachedViews">
						<component :is="Component" />
					</keep-alive>
				</RouterView>
			</a-layout-content>
		</a-layout>
	</a-layout>
</template>


<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router';
import { computed, onMounted, ref, watch, onActivated } from 'vue';

defineOptions({
	name: 'scan-home',
})

// 保存上次选择的子菜单路由名称
const lastVisitedRoute = ref('');

let menuCollapse = ref(false)
let mwidth = 180
const menuWidth = computed(() => {
	return menuCollapse.value? 48:mwidth;
});

const setCollapsed = (val: boolean) => {
	menuCollapse.value = val
};


const paddingStyle = computed(() => {
	const paddingLeft = { paddingLeft: '220px' }
	const paddingTop = { paddingTop: '20px' }
	const paddingRight = { paddingRight: '20px' }
	return { ...paddingLeft, ...paddingTop, ...paddingRight };
});

const route = useRoute()
const router = useRouter()

const selectedKeys = ref<string[]>(["1"])

// 根据当前路由设置 selectedKeys
const updateSelectedKeys = () => {
	const routeName = route.name
	console.log(routeName)
	switch (routeName) {
		case 'scan-active':
			selectedKeys.value = ['1']
			break
		case 'scan-passive':
			selectedKeys.value = ['2']
			break
		case 'scan-plugins':
			selectedKeys.value = ['3']
			break
		default:
			selectedKeys.value = []
	}
}

watch(() => route.name, () => {
	updateSelectedKeys();
	
	// 保存当前子菜单路由名称（如果不是根路由）
	if (route.name && route.name.toString() !== 'scan') {
		lastVisitedRoute.value = route.name.toString();
	}
})

onMounted(() => {
	// 初始化时设置 selectedKeys
	updateSelectedKeys();
})

// 当组件被激活（从缓存中恢复）时调用
onActivated(() => {
	// 如果当前是根路由，且记忆了上一次访问的子路由，则导航过去
	if (route.name === 'scan' && lastVisitedRoute.value) {
		router.push({ name: lastVisitedRoute.value });
	} else if (route.name === 'scan' && !lastVisitedRoute.value) {
		// 如果没有记忆的路由，则使用默认值
		router.push({ name: "scan-active" });
	}
});

const onMenuClick = (key: string) => {
	console.log(key)
	switch (key) {
		case '1':
			router.push({ name: "scan-active" });
			break
		case '2':
			router.push({ name: "scan-passive" });
			break
		case '3':
			router.push({ name: "scan-plugins" });
			break
		default:
			router.push({ name: "scan-active" });
			break

	}
}

// 根据路由meta.keepAlive属性判断是否需要缓存
const cachedViews = computed(() => {
	const cacheList: string[] = [];
	const routes = router.getRoutes();
	routes.forEach(route => {
		if (route.meta && route.meta.keepAlive && route.name) {
			// 添加路由名称
			cacheList.push(route.name.toString());
			// 如果meta中定义了componentName，也添加它
			if (route.meta.componentName) {
				cacheList.push(route.meta.componentName.toString());
			}
		}
	});
	console.log('Scan cached views:', cacheList);
	return cacheList;
});

</script>

<style scoped>
.layout {
	width: 100%;
	height: 100%;
}

.layout-sider {
	position: fixed;
	top: 0;
	left: 0;
	z-index: 99;
	height: 100%;
	transition: all 0.2s cubic-bezier(0.34, 0.69, 0.1, 1);

	&::after {
		position: absolute;
		top: 0;
		right: -1px;
		display: block;
		width: 1px;
		height: 100%;
		background-color: var(--color-border);
		content: '';
	}

	> :deep(.arco-layout-sider-children) {
		overflow-y: hidden;
	}
}

.layout-content {
	overflow-y: hidden;
	transition: padding 0.2s cubic-bezier(0.34, 0.69, 0.1, 1);
}


.menu-wrapper {
	height: 100%;
	overflow: auto;
	overflow-x: hidden;

	:deep(.arco-menu) {
		::-webkit-scrollbar {
			width: 12px;
			height: 4px;
		}

		::-webkit-scrollbar-thumb {
			border: 4px solid transparent;
			background-clip: padding-box;
			border-radius: 7px;
			background-color: var(--color-text-4);
		}

		::-webkit-scrollbar-thumb:hover {
			background-color: var(--color-text-3);
		}
	}
}
</style>