import { invoke } from '@tauri-apps/api/core';

/**
 * 扫描插件相关API
 */

// 获取所有扫描插件
export async function getScanPlugins() {
  return invoke('list_scan_plugins');
}

// 获取单个扫描插件详情
export async function getScanPlugin(pluginId: string) {
  return invoke('get_scan_plugin', { pluginId });
}

// 重新加载所有扫描插件
export async function reloadScanPlugins() {
  return invoke('reload_scan_plugins');
}

// 验证插件脚本
export async function validatePluginScript(params: { script: string }) {
  return invoke('validate_scan_plugin', { script: params.script });
}

// 上传扫描插件内容
export async function uploadScanPlugin(params: { filename: string, content: string }) {
  return invoke('upload_scan_plugin_content', { 
    filename: params.filename,
    content: params.content 
  });
}

// 删除扫描插件
export async function deleteScanPlugin(pluginId: string) {
  return invoke('delete_scan_plugin', { pluginId });
}

// 更新扫描插件
export async function updateScanPlugin(pluginId: string, params: { script: string }) {
  return invoke('update_scan_plugin', { 
    pluginId,
    script: params.script 
  });
}

// 执行扫描插件
export async function executeScanPlugin(params: { 
  plugin_id: string, 
  target: string,
  params?: Record<string, any>
}) {
  return invoke('execute_scan_plugin', { 
    plugin_id: params.plugin_id,
    target: params.target,
    params: params.params
  });
}

// ... rest of the file remains unchanged ...

 