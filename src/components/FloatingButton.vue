<template>
  <div 
    ref="draggableButton" 
    class="floating-button" 
    :style="buttonStyle"
    @mousedown="startDrag"
  >
    <a-button status="warning" :shape="'circle'" @click="openRepeter">
      <template #icon>
        <icon-live-broadcast />
      </template>
    </a-button>
  </div>

  <!-- //drawer宽带用百分比设置，占主页面的90% -->
  <a-drawer :header=false v-model:visible="visible" :width="`calc(90%)`" :title="$t('settings.title')" :footer="false">
    <div>
      <a-tabs :editable="true" show-add-button auto-switch v-model:activeKey="activeTabKey" @add="handleAdd" @delete="handleDelete" >
        <a-tab-pane v-for="tab in tabs" :key="tab.key" :title="tab.title" :closable="tab.closable">
          <component :is="tab.component" />
        </a-tab-pane>
      </a-tabs>
    </div>
  </a-drawer>
  
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, onBeforeUnmount, markRaw } from 'vue';
import Repeater from '@/components/repeater/components/repeater.vue';

const visible = ref(false);
const activeTabKey = ref('1');

// 定义标签页数据结构
interface TabItem {
  key: string;
  title: string;
  component: any; // 使用any类型，避免类型导入问题
  closable: boolean;
}

// 标签页数据
const tabs = ref<TabItem[]>([
  {
    key: '1',
    title: 'repeater',
    component: markRaw(Repeater),
    closable: false
  }
]);

const tabCount = ref(1);

const openRepeter = () => {
  visible.value = true;
};

const handleAdd = () => {
  tabCount.value++;
  const newKey = tabCount.value.toString();
  
  tabs.value.push({
    key: newKey,
    title: `repeater-${newKey}`,
    component: markRaw(Repeater),
    closable: true
  });
  
  // 切换到新创建的标签页
  activeTabKey.value = newKey;
  
  // 将标签页信息保存到localStorage
  saveTabsToStorage();
};

const handleDelete = (key: string | number, ev?: Event) => {
  tabCount.value--;
  // 将key转换为字符串确保类型安全
  const tabKey = String(key);
  
  // 如果要删除的是当前活动标签页，需要先切换到其他标签页
  if (activeTabKey.value === tabKey) {
    const index = tabs.value.findIndex(item => item.key === tabKey);
    if (index > 0) {
      // 如果不是第一个标签页，切换到前一个
      activeTabKey.value = tabs.value[index - 1].key;
    } else if (tabs.value.length > 1) {
      // 如果是第一个标签页且有其他标签页，切换到下一个
      activeTabKey.value = tabs.value[1].key;
    }
  }
  
  // 删除标签页
  tabs.value = tabs.value.filter(item => item.key !== tabKey);
  
  // 保存到localStorage
  saveTabsToStorage();
};

// 保存标签页信息到localStorage
const saveTabsToStorage = () => {
  // 只保存必要的信息，避免保存组件导致序列化错误
  const tabsInfo = tabs.value.map(tab => ({
    key: tab.key,
    title: tab.title,
    closable: tab.closable
  }));
  
  localStorage.setItem('repeaterTabs', JSON.stringify({
    tabs: tabsInfo,
    activeTabKey: activeTabKey.value,
    tabCount: tabCount.value
  }));
};

// 从localStorage加载标签页信息
const loadTabsFromStorage = () => {
  const tabsInfo = localStorage.getItem('repeaterTabs');
  if (tabsInfo) {
    try {
      const parsedInfo = JSON.parse(tabsInfo);
      
      if (parsedInfo.tabs && Array.isArray(parsedInfo.tabs)) {
        // 重建标签页，为每个标签页附加组件
        tabs.value = parsedInfo.tabs.map((tab: {key: string, title: string, closable: boolean}) => ({
          ...tab,
          component: markRaw(Repeater)
        }));
        
        activeTabKey.value = parsedInfo.activeTabKey || '1';
        tabCount.value = parsedInfo.tabCount || 1;
      }
    } catch (error) {
      console.error('加载标签页信息失败:', error);
    }
  }
};

// 按钮位置状态
const position = reactive({
  x: localStorage.getItem('floatingButtonX') ? Number(localStorage.getItem('floatingButtonX')) : window.innerWidth - 100,
  y: localStorage.getItem('floatingButtonY') ? Number(localStorage.getItem('floatingButtonY')) : 100
});

// 按钮样式，包括位置
const buttonStyle = ref({
  left: `${position.x}px`,
  top: `${position.y}px`
});

// 拖拽状态
const isDragging = ref(false);
const dragOffset = reactive({ x: 0, y: 0 });
const draggableButton = ref<HTMLElement | null>(null);

// 开始拖拽
const startDrag = (e: MouseEvent) => {
  if (draggableButton.value) {
    isDragging.value = true;
    dragOffset.x = e.clientX - position.x;
    dragOffset.y = e.clientY - position.y;
    
    document.addEventListener('mousemove', onDrag);
    document.addEventListener('mouseup', stopDrag);
    
    // 防止拖动时选中文本
    e.preventDefault();
  }
};

// 拖拽过程
const onDrag = (e: MouseEvent) => {
  if (isDragging.value) {
    // 计算新位置
    position.x = e.clientX - dragOffset.x;
    position.y = e.clientY - dragOffset.y;
    
    // 限制在可视区域内
    position.x = Math.max(0, Math.min(position.x, window.innerWidth - 60));
    position.y = Math.max(0, Math.min(position.y, window.innerHeight - 60));
    
    // 更新样式
    buttonStyle.value = {
      left: `${position.x}px`,
      top: `${position.y}px`
    };
  }
};

// 停止拖拽
const stopDrag = () => {
  if (isDragging.value) {
    isDragging.value = false;
    document.removeEventListener('mousemove', onDrag);
    document.removeEventListener('mouseup', stopDrag);
    
    // 保存位置到本地存储
    localStorage.setItem('floatingButtonX', String(position.x));
    localStorage.setItem('floatingButtonY', String(position.y));
  }
};

// 窗口大小改变时确保按钮在可视区域内
const handleResize = () => {
  if (position.x > window.innerWidth - 60) {
    position.x = window.innerWidth - 60;
  }
  if (position.y > window.innerHeight - 60) {
    position.y = window.innerHeight - 60;
  }
  buttonStyle.value = {
    left: `${position.x}px`,
    top: `${position.y}px`
  };
};

onMounted(() => {
  window.addEventListener('resize', handleResize);
  // 加载标签页信息
  loadTabsFromStorage();
});

onBeforeUnmount(() => {
  window.removeEventListener('resize', handleResize);
  document.removeEventListener('mousemove', onDrag);
  document.removeEventListener('mouseup', stopDrag);
  // 保存标签页状态
  saveTabsToStorage();
});
</script>

<style scoped>
.floating-button {
  position: fixed;
  z-index: 9999;
  cursor: move;
  user-select: none;
  filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.2));
}
</style> 