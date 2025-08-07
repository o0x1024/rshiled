<template>
	<a-layout class="layout">
		<a-layout>
			<a-layout-sider class="layout-sider" @collapse="setCollapsed"  :width="menuWidth" :style="{ paddingTop: '60px' }">
				<div class="menu-wrapper">
					<a-menu 
						:style="{ height: 'calc(100% - 0px)' }"  
						@collapse="setCollapsed" 
						@menu-item-click="onMenuClick" 
						mode="pop"
						:selected-keys="selectedKeys"
						showCollapseButton>
						<a-menu-item key="1">
							<template #icon><icon-dashboard></icon-dashboard></template>
							{{ $t('asm.overview') }}
						</a-menu-item>
						<a-menu-item key="2">
							<template #icon><icon-book /></template>
							{{ $t('asm.task') }}
						</a-menu-item>
						<a-menu-item key="3">
							<template #icon><icon-mobile /></template>
							{{ $t('asm.root-domain') }}
						</a-menu-item>
						<a-menu-item key="4">
							<template #icon><icon-highlight /></template>
							{{ $t('asm.domain') }}
						</a-menu-item>
						<a-menu-item key="5">
							<template #icon><icon-fire /></template>
							{{ $t('asm.ip') }}
						</a-menu-item>
						<a-menu-item key="6">
							<template #icon><icon-desktop /></template>
							{{ $t('asm.port') }}
						</a-menu-item>
						<a-menu-item key="7">
							<template #icon><icon-command /></template>
							{{ $t('asm.website') }}
						</a-menu-item>
						<a-menu-item key="8">
							<template #icon><icon-command /></template>
							{{ $t('asm.api') }}
						</a-menu-item>
						<a-menu-item key="9">
							<template #icon><icon-google-circle-fill /></template>
							{{ $t('asm.web-component') }}
						</a-menu-item>
						<a-menu-item key="10">
							<template #icon><icon-bug /></template>
							{{ $t('asm.risk') }}
						</a-menu-item>
						<a-divider />
						<a-menu-item key="11">
							<template #icon><icon-pushpin /></template>
							{{ $t('asm.plugin') }}
						</a-menu-item>
						<a-divider />
						<a-sub-menu key="visualization">
							<template #icon><icon-command /></template>
							<template #title>{{ $t('asm.visualization') }}</template>
							<a-menu-item key="12">
								<template #icon><icon-relation /></template>
								{{ $t('asm.visualization.asset_graph') }}
							</a-menu-item>
							<a-menu-item key="13">
								<template #icon><icon-mind-mapping /></template>
								{{ $t('asm.visualization.risk_heatmap') }}
							</a-menu-item>
							<a-menu-item key="14">
								<template #icon><icon-file-pdf /></template>
								{{ $t('asm.visualization.compliance_report') }}
							</a-menu-item>
						</a-sub-menu>
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
import { computed, ref, watch, onMounted, onActivated } from 'vue';

defineOptions({
	name: 'asm-home',
})

// 保存上次选择的子菜单路由名称
const lastVisitedRoute = ref('');

// 打印当前路由信息，用于调试
onMounted(() => {
  const routes = router.getRoutes();
  routes.forEach(route => {
    if (route.name && route.name.toString().startsWith('asm-')) {
      console.log('Route:', route.name, 'Meta:', route.meta);
    }
  });
});

let menuCollapse = ref(false)

let mwidth = 180
const menuWidth = computed(() => {
	return menuCollapse.value? 48:mwidth;
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
  console.log('ASM cached views:', cacheList);
  return cacheList;
});

const setCollapsed = (val: boolean) => {
	menuCollapse.value = val
};

const router = useRouter()
const route = useRoute()

const selectedKeys = ref<string[]>([])

// 根据当前路由设置 selectedKeys
const updateSelectedKeys = () => {
  const routeName = route.name
  switch (routeName) {
    case 'asm-dashboard':
      selectedKeys.value = ['1']
      break
    case 'asm-scan-object':
      selectedKeys.value = ['2']
      break
    case 'asm-root-domain':
      selectedKeys.value = ['3']
      break
    case 'asm-domain':
      selectedKeys.value = ['4']
      break
    case 'asm-ip':
      selectedKeys.value = ['5']
      break
    case 'asm-port':
      selectedKeys.value = ['6']
      break
    case 'asm-website':
      selectedKeys.value = ['7']
      break
	case 'asm-api':
      selectedKeys.value = ['8']
      break
    case 'asm-web-component':
      selectedKeys.value = ['9']
      break
    case 'asm-risk':
      selectedKeys.value = ['10']
      break
	case 'asm-plugin':
      selectedKeys.value = ['11']
      break
	case 'asm-visualization-asset-graph':
      selectedKeys.value = ['12']
      break
	case 'asm-visualization-risk-heatmap':
      selectedKeys.value = ['13']
      break
	case 'asm-visualization-compliance-report':
      selectedKeys.value = ['14']
      break
    default:
      selectedKeys.value = []
  }
}

// 监听路由变化，更新 selectedKeys
watch(() => route.name, () => {
  updateSelectedKeys();
  
  // 保存当前子菜单路由名称（如果不是根路由）
  if (route.name && route.name.toString() !== 'asm') {
    lastVisitedRoute.value = route.name.toString();
  }
})

// 初始化时设置 selectedKeys
updateSelectedKeys();

// 当组件被激活（从缓存中恢复）时调用
onActivated(() => {
  // 如果当前是根路由，且记忆了上一次访问的子路由，则导航过去
  if (route.name === 'asm' && lastVisitedRoute.value) {
    router.push({ name: lastVisitedRoute.value });
  } else if (route.name === 'asm' && !lastVisitedRoute.value) {
    // 如果没有记忆的路由，则使用默认值
    router.push({ name: "asm-scan-object" });
  }
});

const paddingStyle = computed(() => {
	const paddingLeft = { paddingLeft: menuCollapse.value? '68px':'220px' }
	const paddingTop = { paddingTop: '20px' }
	const paddingRight = { paddingRight: '20px' }
	return { ...paddingLeft, ...paddingTop, ...paddingRight };
});

const onMenuClick = (key: string) => {
	switch (key) {
		case '1':
			router.push({ name: "asm-dashboard" });
			break
		case '2':
			router.push({ name: "asm-scan-object" });
			break
		case '3':
			router.push({ name: "asm-root-domain" });
			break
		case '4':
			router.push({ name: "asm-domain" });
			break
		case '5':
			router.push({ name: "asm-ip" });
			break
		case '6':
			router.push({ name: "asm-port" });
			break
		case '7':
			router.push({ name: "asm-website" });
			break
		case '8':
			router.push({ name: "asm-api" });
			break
		case '9':
			router.push({ name: "asm-web-component" });
			break
		case '10':
			router.push({ name: "asm-risk" });
			break
		case '11':
			router.push({ name: "asm-plugin" });
			break
		case '12':
			router.push({ name: "asm-visualization-asset-graph" });
			break
		case '13':
			router.push({ name: "asm-visualization-risk-heatmap" });
			break
		case '14':
			router.push({ name: "asm-visualization-compliance-report" });
			break
	}
}
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