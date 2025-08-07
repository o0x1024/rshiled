<template>
	<a-layout class="layout">
		<a-layout>
			<a-layout-sider class="layout-sider" @collapse="setCollapsed" :width="menuWidth"
				:style="{ paddingTop: '60px' }">
				<div class="menu-wrapper">
					<a-menu :style="{ height: 'calc(100% - 0px)' }" @collapse="setCollapsed"
						@menu-item-click="onMenuClick" mode="pop" showCollapseButton :selected-keys="selectedKeys">
						<a-menu-item key="1">
							<template #icon><icon-book /></template>
							{{ $t('setting.scan') }}
						</a-menu-item>
						<a-menu-item key="2">
							<template #icon><icon-dashboard></icon-dashboard></template>
							{{ $t('setting.asm') }}
						</a-menu-item>

					</a-menu>
				</div>
			</a-layout-sider>
			<a-layout-content class="layout-content" :style="paddingStyle">
				<RouterView />
			</a-layout-content>
		</a-layout>
	</a-layout>
</template>


<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRouter, useRoute } from 'vue-router';

let menuCollapse = ref(false)
let mwidth = 180
const menuWidth = computed(() => {
	return menuCollapse.value ? 48 : mwidth;
});

const setCollapsed = (val: boolean) => {
	console.log(val)
	menuCollapse.value = val
};
const route = useRoute()
const router = useRouter()

const paddingStyle = computed(() => {
	const paddingLeft = { paddingLeft: '220px' }
	const paddingTop = { paddingTop: '20px' }
	const paddingRight = { paddingRight: '20px' }
	return { ...paddingLeft, ...paddingTop, ...paddingRight };
});

const selectedKeys = ref<string[]>([])

// 根据当前路由设置 selectedKeys
const updateSelectedKeys = () => {
	const routeName = route.name
	switch (routeName) {
		case 'setting-scan':
			selectedKeys.value = ['1']
			break
		case 'setting-asm':
			selectedKeys.value = ['2']
			break

		default:
			selectedKeys.value = []
	}
}
// 监听路由变化，更新 selectedKeys
watch(() => route.name, () => {
	updateSelectedKeys()
})

onMounted(() => {
	// 初始化时设置 selectedKeys
	updateSelectedKeys()

})

const onMenuClick = (key: string) => {
	console.log(key)
	switch (key) {
		case '1':
			router.push({ name: "setting-scan" });
			break
		case '2':
			router.push({ name: "setting-asm" });
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