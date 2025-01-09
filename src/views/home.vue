<template>
	<a-layout class="layout" >
		<div  class="layout-navbar">
		  <NavBar />
		</div>
		<a-layout>
		  <a-layout>
			<a-layout :style="paddingStyle">
			  <a-layout-content>
				<router-view >
				  </router-view>
			  </a-layout-content>
			</a-layout>
		  </a-layout>
		</a-layout>
	  </a-layout>
</template>

<script setup lang="ts">
import NavBar from '@/components/navbar/index.vue';
import { useAppStore } from '@/store';
import { computed } from 'vue';


const navbarHeight = `60px`;
const appStore = useAppStore();
const navbar = computed(() => appStore.navbar);
const menuWidth = computed(() => {
    return appStore.menuCollapse ? 48 : appStore.menuWidth;
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