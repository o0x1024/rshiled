import { invoke } from '@tauri-apps/api/core';


// Plugin parameter interface
export interface PluginParameter {
  name: string;
  description: string;
  default_value: string;
  required: boolean;
  parameter_type: string; // string, number, boolean, etc.
}

// Plugin interface
export interface Plugin {
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  category: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  code: string;
  script_type: string; // javascript, python, etc.
  targets: string[]; // what kind of targets this plugin can scan
  parameters: PluginParameter[];
}

// Plugin list options interface
export interface PluginListOptions {
  category?: string;
  enabled_only?: boolean;
  search_term?: string;
  limit?: number;
  offset?: number;
}

// Plugin list result interface
export interface PluginListResult {
  plugins: Plugin[];
  total: number;
}

// Success response interface
export interface SuccessResponse {
  success: boolean;
  message: string;
}

/**
 * Get a list of plugins
 * @param options Options for filtering and pagination
 * @returns A promise that resolves to a list of plugins
 */
export async function getPlugins(options?: PluginListOptions): Promise<PluginListResult> {
  try {
    return await invoke<PluginListResult>('get_plugins', { options });
  } catch (error) {
    console.error('Error fetching plugins:', error);
    throw error;
  }
}

/**
 * Get a single plugin by ID
 * @param id The plugin ID
 * @returns A promise that resolves to a plugin
 */
export async function getPlugin(id: string): Promise<Plugin> {
  try {
    return await invoke<Plugin>('get_plugin', { id });
  } catch (error) {
    console.error(`Error fetching plugin ${id}:`, error);
    throw error;
  }
}

/**
 * Save a plugin
 * @param plugin The plugin to save
 * @returns A promise that resolves to a success response
 */
export async function savePlugin(plugin: Plugin): Promise<SuccessResponse> {
  try {
    return await invoke<SuccessResponse>('save_plugin', { plugin });
  } catch (error) {
    console.error('Error saving plugin:', error);
    throw error;
  }
}

/**
 * Toggle a plugin's enabled status
 * @param id The plugin ID
 * @param enabled Whether the plugin should be enabled
 * @returns A promise that resolves to a success response
 */
export async function togglePlugin(id: string, enabled: boolean): Promise<SuccessResponse> {
  try {
    return await invoke<SuccessResponse>('toggle_plugin', { id, enabled });
  } catch (error) {
    console.error(`Error toggling plugin ${id}:`, error);
    throw error;
  }
}

/**
 * Delete a plugin
 * @param id The plugin ID
 * @returns A promise that resolves to a success response
 */
export async function deletePlugin(id: string): Promise<SuccessResponse> {
  try {
    return await invoke<SuccessResponse>('delete_plugin', { id });
  } catch (error) {
    console.error(`Error deleting plugin ${id}:`, error);
    throw error;
  }
}

/**
 * Get a list of plugin categories
 * @returns A promise that resolves to a list of categories
 */
export async function getPluginCategories(): Promise<string[]> {
  try {
    return await invoke<string[]>('get_plugin_categories');
  } catch (error) {
    console.error('Error fetching plugin categories:', error);
    throw error;
  }
}

/**
 * Create a new plugin
 * @returns A newly created plugin with default values
 */
export function createNewPlugin(): Plugin {
  return {
    id: '',
    name: '',
    version: '1.0.0',
    author: '',
    description: '',
    category: 'Web',
    enabled: false,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    code: 'function scan(target) {\n  // Your scanning logic here\n  return {\n    vulnerable: false,\n    details: "No vulnerabilities found"\n  };\n}',
    script_type: 'javascript',
    targets: ['web'],
    parameters: []
  };
}

/**
 * Create a new plugin parameter
 * @returns A newly created parameter with default values
 */
export function createNewParameter(): PluginParameter {
  return {
    name: '',
    description: '',
    default_value: '',
    required: false,
    parameter_type: 'string'
  };
} 