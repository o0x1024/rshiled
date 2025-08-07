<template>
  <div class="navbar">
    <div class="left-side">
      <a-space>
        <img alt="logo" width="35" height="35" style="" src="../../assets/logo.png" />
        <a-typography-title :style="{ margin: 0, fontSize: '18px' }" :heading="5">
          rshiled
        </a-typography-title>
        <icon-menu-fold v-if="!topMenu && appStore.device === 'mobile'" style="font-size: 22px; cursor: pointer" />
      </a-space>
    </div>
    <div class="center-side">
      <Menu v-if="topMenu" />
    </div>
    <ul class="right-side">
      <li>
        <a-tooltip :content="$t('settings.search')">
          <a-button class="nav-btn" type="outline" :shape="'circle'">
            <template #icon>
              <icon-search />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <li>
        <a-tooltip :content="$t('settings.language')">
          <a-button class="nav-btn" type="outline" :shape="'circle'" @click="setDropDownVisible">
            <template #icon>
              <icon-language />
            </template>
          </a-button>
        </a-tooltip>
        <a-dropdown trigger="click" @select="changeLocale as any">
          <div ref="triggerBtn" class="trigger-btn"></div>
          <template #content> 
            <a-doption v-for="item in locales" :key="item.value" :value="item.value">
              <template #icon>
                <icon-check v-show="item.value === currentLocale" />
              </template>
              {{ item.label }}
            </a-doption> 
          </template>
        </a-dropdown>
      </li>
      <li>
        <a-tooltip :content="theme === 'light'
            ? $t('settings.navbar.theme.toDark')
            : $t('settings.navbar.theme.toLight')
          ">
          <a-button class="nav-btn" type="outline" :shape="'circle'" @click="handleToggleTheme">
            <template #icon>
              <icon-moon-fill v-if="theme === 'dark'" />
              <icon-sun-fill v-else />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <li>
        <a-tooltip :content="$t('settings.navbar.log')">
          <a-button class="nav-btn" type="outline" :shape="'circle'" @click="openLogWindow">
            <template #icon>
              <icon-info-circle-fill />
            </template>
          </a-button>
        </a-tooltip>
      </li>
      <!-- <li>
        <a-tooltip :content="$t('settings.title')">
          <a-button class="nav-btn" type="outline" :shape="'circle'" @click="setVisible">
            <template #icon>
              <icon-settings />
            </template>
          </a-button>
        </a-tooltip>
      </li>-->
      <li> 

      </li>
    </ul>
  </div>
</template>

<script lang="ts" setup>
import { computed, ref } from 'vue';
import { useDark, useToggle } from '@vueuse/core';
import { useAppStore } from '@/store';
import { LOCALE_OPTIONS } from '@/locale';
import useLocale from '@/hooks/locale';
import Menu from '@/components/menu/index.vue';
import { useI18n } from 'vue-i18n';
import { Message } from '@arco-design/web-vue';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow, getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { emit } from '@tauri-apps/api/event';

const appStore = useAppStore();
const { t } = useI18n();
const { currentLocale, changeLocale } = useLocale();

const locales = [...LOCALE_OPTIONS];

const theme = computed(() => {
  return appStore.theme;
});
const topMenu = computed(() => appStore.topMenu && appStore.menu);
const isDark = useDark({
  selector: 'body',
  attribute: 'arco-theme',
  valueDark: 'dark',
  valueLight: 'light',
  storageKey: 'arco-theme',
  onChanged(dark: boolean) {
    // overridden default behavior
    appStore.toggleTheme(dark);
  },
});
const toggleTheme = useToggle(isDark);
const handleToggleTheme = () => {
  toggleTheme();
  
  // 通知日志查看器窗口主题变化
  notifyLogViewerThemeChange();
};

// 通知日志查看器窗口主题变化
async function notifyLogViewerThemeChange() {
  try {
    // 使用全局事件发送主题变化通知
    await emit('theme-changed', {
      isDark: isDark.value
    });
  } catch (error) {
    console.error('通知日志查看器主题变化失败:', error);
  }
}

// 打开日志窗口
async function openLogWindow() {
  try {
    // 获取日志文件路径
    const logPath = await invoke('get_log_file_path');
    
    // 检查窗口是否已存在
    const windows = await getAllWebviewWindows();
    const existingWindow = windows.find((w: WebviewWindow) => w.label === 'log-viewer');
    
    if (existingWindow) {
      // 如果窗口已存在，则聚焦并显示它
      try {
        // 短暂设置为置顶，确保显示在前面
        await existingWindow.setAlwaysOnTop(true);
        // 设置焦点并显示
        await existingWindow.setFocus();
        await existingWindow.show();
        
        // 更新日志信息
        await emit('log-data', {
          logPath,
          isDark: isDark.value
        });
        
        // 一段时间后取消置顶
        setTimeout(async () => {
          await existingWindow.setAlwaysOnTop(false);
        }, 300);
      } catch (err) {
        console.error('设置已存在日志窗口焦点失败:', err);
        Message.error(`设置窗口焦点失败: ${err}`);
      }
    } else {
      // 如果窗口不存在，创建新窗口
      const logViewerWindow = new WebviewWindow('log-viewer', {
        url: '/log-viewer',
        title: t('settings.log.title'),
        width: 1200,
        height: 700,
        resizable: true,
        focus: true,
        alwaysOnTop: true,
        skipTaskbar: false
      });

      // 创建后设置窗口焦点
      logViewerWindow.once('tauri://created', async () => {
        try {
          // 设置焦点
          await logViewerWindow.setFocus();
          // 确保窗口显示
          await logViewerWindow.show();
          
          // 等待一小段时间确保窗口已完成加载
          setTimeout(async () => {
            await emit('log-data', {
              logPath,
              isDark: isDark.value
            });
            // 短暂设置为置顶，然后取消置顶，确保窗口显示在前面
            setTimeout(async () => {
              await logViewerWindow.setAlwaysOnTop(false);
            }, 300);
          }, 500);
        } catch (err) {
          console.error('设置日志窗口焦点失败:', err);
        }
      });
    }
  } catch (error) {
    Message.error(`无法打开日志查看器: ${error}`);
  }
}

const triggerBtn = ref();

const setDropDownVisible = () => {
  const event = new MouseEvent('click', {
    view: window,
    bubbles: true,
    cancelable: true,
  });
  triggerBtn.value.dispatchEvent(event);
};
</script>

<style scoped lang="less">
.navbar {
  display: flex;
  justify-content: space-between;
  height: 100%;
  background-color: var(--color-bg-2);
  border-bottom: 1px solid var(--color-border);
}

.left-side {
  display: flex;
  align-items: center;
  padding-left: 20px;
}

.center-side {
  flex: 1;
}

.right-side {
  display: flex;
  padding-right: 20px;
  list-style: none;

  :deep(.locale-select) {
    border-radius: 20px;
  }

  li {
    display: flex;
    align-items: center;
    padding: 0 10px;
  }

  a {
    color: var(--color-text-1);
    text-decoration: none;
  }

  .nav-btn {
    border-color: rgb(var(--gray-2));
    color: rgb(var(--gray-8));
    font-size: 16px;
  }

  .trigger-btn,
  .ref-btn {
    position: absolute;
    bottom: 14px;
  }

  .trigger-btn {
    margin-left: 14px;
  }
}

.log-container {
  max-height: 500px;
  overflow: auto;

  .code-editor-container {
    border: 1px solid var(--color-border-2);
    border-radius: 4px;
    overflow: hidden;

    :deep(.cm-editor) {
      height: 500px;
      font-size: 12px;
      background-color: var(--color-bg-2);
      color: var(--color-text-1);
    }

    :deep(.cm-gutters) {
      border: none;
      background-color: var(--color-bg-2);
      color: var(--color-text-3);
    }

    :deep(.cm-activeLineGutter) {
      background-color: var(--color-fill-2);
    }

    :deep(.cm-activeLine) {
      background-color: var(--color-fill-2);
    }

    :deep(.cm-content) {
      font-family: monospace;
    }

    :deep(.cm-lineNumbers) {
      color: var(--color-text-3);
    }

    :deep(.cm-cursor) {
      border-left: 2px solid var(--color-text-1);
    }

    :deep(.cm-selectionBackground) {
      background-color: var(--color-fill-2);
    }

    &.dark-mode {
      :deep(.cm-editor) {
        background-color: #1e1e1e;
        color: #d4d4d4;
      }

      :deep(.cm-gutters) {
        background-color: #262626;
        border-right: 1px solid #333;
      }

      :deep(.cm-activeLineGutter) {
        background-color: #2c2c2c;
      }

      :deep(.cm-activeLine) {
        background-color: #2c2c2c;
      }

      :deep(.cm-lineNumbers) {
        color: #666;
      }

      :deep(.cm-cursor) {
        border-left: 2px solid #fff;
      }

      :deep(.cm-selectionBackground) {
        background-color: #2c2c2c;
      }

      :deep(.cm-content) {
        color: #d4d4d4;
      }
    }
  }
}

.log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.log-path {
  font-size: 12px;
  color: var(--color-text-2);
}
</style>

<style lang="less">
.message-popover {
  .arco-popover-content {
    margin-top: 0;
  }
}
</style>
