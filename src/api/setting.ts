import { invoke } from '@tauri-apps/api/core';
import { AsmConfig } from '@/views/setting/types/config';

export interface SuccessResponse {
  success: boolean;
  message: string;
}

export const getAsmConfig = async (): Promise<AsmConfig> => {
  try {
    return await invoke<AsmConfig>('get_asm_config');
  } catch (error) {
    console.error('Failed to get ASM configuration:', error);
    return {
      dns_collection_brute_status: false,
      dns_collection_plugin_status: false,
      is_buildin: false,
      proxy: '',
      user_agent: '',
      http_headers: [],
      http_timeout: 30,
      thread_num: 10
    };
  }
};

export const updateAsmConfig = async (config: AsmConfig): Promise<boolean> => {
  try {
    const response = await invoke<SuccessResponse>('update_asm_config', { config });
    return response.success;
  } catch (error) {
    console.error('Failed to update ASM configuration:', error);
    return false;
  }
}; 