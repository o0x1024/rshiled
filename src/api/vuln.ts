import { invoke } from '@tauri-apps/api/core';
import { 
  VulnPlugin, 
  PluginPolicy, 
  ScanTask, 
  ScanResult, 
  ExploitParams, 
  ExploitResult, 
  PluginStatus
} from '@/views/vuln/types/plugin';

/**
 * Plugin Related APIs
 */

// Get all plugins
export async function getAllPlugins(): Promise<VulnPlugin[]> {
  return invoke('list_rhai_plugins');
}

// Get a plugin by ID
export async function getPlugin(id: number): Promise<VulnPlugin | null> {
  return invoke('get_vuln_plugin', { id });
}

// Create a new plugin
export async function createPlugin(plugin: VulnPlugin): Promise<boolean> {
  try {
    const result = await invoke('add_vuln_plugin', {
      name: plugin.name,
      pluginType: plugin.rtype,
      description: plugin.description,
      author: plugin.author,
      version: plugin.version,
      references: plugin.references,
      severity: plugin.severity,
      script: plugin.script,
      params: plugin.params,
      resultFields: plugin.resultFields || []
    });
    return result !== null && result !== undefined;
  } catch (error) {
    console.error('Error creating plugin:', error);
    return false;
  }
}

// Update a plugin
export async function updatePlugin(plugin: VulnPlugin): Promise<boolean> {
  try {
    if (!plugin.id) return false;
    
    await invoke('update_vuln_plugin', {
      id: plugin.id,
      name: plugin.name,
      pluginType: plugin.rtype,
      description: plugin.description,
      author: plugin.author,
      version: plugin.version,
      references: plugin.references,
      severity: plugin.severity,
      status: plugin.status === PluginStatus.ENABLED ? 1 : 0,
      script: plugin.script,
      params: plugin.params,
      resultFields: plugin.resultFields || []
    });
    return true;
  } catch (error) {
    console.error('Error updating plugin:', error);
    return false;
  }
}

// Delete a plugin
export async function deletePlugin(id: number): Promise<boolean> {
  try {
    await invoke('delete_vuln_plugin', { id });
    return true;
  } catch (error) {
    console.error('Error deleting plugin:', error);
    return false;
  }
}

// Toggle plugin status
export async function togglePluginStatus(id: number, _status: PluginStatus): Promise<boolean> {
  try {
    await invoke('toggle_plugin_status', { id });
    return true;
  } catch (error) {
    console.error('Error toggling plugin status:', error);
    return false;
  }
}

// Get plugin types
export async function getPluginTypes(): Promise<string[]> {
  return invoke('get_plugin_types');
}

// Get severity levels
export async function getSeverityLevels(): Promise<string[]> {
  return invoke('get_severity_levels');
}

/**
 * Policy Related APIs
 */

// Get all policies
export async function getAllPolicies(): Promise<PluginPolicy[]> {
  return invoke('list_plugin_policies');
}

// Get a policy by ID
export async function getPolicy(id: number): Promise<PluginPolicy | null> {
  return invoke('get_plugin_policy', { id });
}

// Create a new policy
export async function createPolicy(policy: PluginPolicy): Promise<boolean> {
  try {
    await invoke('add_plugin_policy', {
      name: policy.name,
      description: policy.description,
      plugins: policy.plugins
    });
    return true;
  } catch (error) {
    console.error('Error creating policy:', error);
    return false;
  }
}

// Update a policy
export async function updatePolicy(policy: PluginPolicy): Promise<boolean> {
  try {
    if (!policy.id) return false;
    
    await invoke('update_plugin_policy', {
      id: policy.id,
      name: policy.name,
      description: policy.description,
      plugins: policy.plugins
    });
    return true;
  } catch (error) {
    console.error('Error updating policy:', error);
    return false;
  }
}

// Delete a policy
export async function deletePolicy(id: number): Promise<boolean> {
  try {
    await invoke('delete_plugin_policy', { id });
    return true;
  } catch (error) {
    console.error('Error deleting policy:', error);
    return false;
  }
}

/**
 * Scan Task Related APIs
 */

// Create a new scan task
export async function createScanTask(task: Partial<ScanTask>): Promise<number> {
  try {
    return await invoke('create_scan_task', {
      name: task.name,
      policy_id: task.policyId,
      targets: task.targets
    });
  } catch (error) {
    console.error('Error creating scan task:', error);
    throw error;
  }
}

// Start a scan task
export async function startScanTask(id: number): Promise<boolean> {
  try {
    await invoke('start_scan_task', { id });
    return true;
  } catch (error) {
    console.error('Error starting scan task:', error);
    return false;
  }
}

// Stop a scan task
export async function stopScanTask(id: number): Promise<boolean> {
  try {
    await invoke('stop_scan_task', { id });
    return true;
  } catch (error) {
    console.error('Error stopping scan task:', error);
    return false;
  }
}

// Delete a scan task
export async function deleteScanTask(id: number): Promise<boolean> {
  try {
    await invoke('delete_scan_task', { id });
    return true;
  } catch (error) {
    console.error('Error deleting scan task:', error);
    return false;
  }
}

// Get all scan tasks
export async function getAllScanTasks(): Promise<ScanTask[]> {
  return invoke('list_scan_tasks');
}

// Get a scan task by ID
export async function getScanTask(id: number): Promise<ScanTask | null> {
  return invoke('get_scan_task', { id });
}

// Get results for a scan task
export async function getTaskResults(taskId: number): Promise<ScanResult[]> {
  return invoke('get_task_results', { taskId });
}

// Get a specific scan result
export async function getScanResult(id: number): Promise<ScanResult | null> {
  return invoke('get_scan_result', { id });
}

/**
 * Exploitation Related APIs
 */

// Exploit a vulnerability
export async function exploitVulnerability(params: ExploitParams): Promise<ExploitResult> {
  try {
    console.log("Exploiting vulnerability with params:", JSON.stringify(params));
    
    const result = await invoke('execute_rhai_plugin', { 
      params: {
        plugin_name: params.plugin_id?.toString(),
        plugin_type: params.plugin_id?.toString().split(':')[0],
        target: params.target,
        custom_params: params.custom_params || {},
        include_http_data: true  // Always request HTTP data
      }
    });
    
    // Process the result to ensure all expected fields are present
    const enhancedResult = result as ExploitResult;
    console.log("Raw exploit result:", JSON.stringify(enhancedResult));
    
    // Ensure backward compatibility
    if (enhancedResult.details && !enhancedResult.message) {
      enhancedResult.message = enhancedResult.details;
    } else if (!enhancedResult.details && enhancedResult.message) {
      enhancedResult.details = enhancedResult.message;
    }
    
    // Ensure HTTP request data is available
    if (!enhancedResult.request) {
      enhancedResult.request = '# HTTP request data not available from backend';
      console.warn("No HTTP request data found in exploit result");
    } else {
      console.log(`HTTP request data found: ${enhancedResult.request.length} characters`);
    }
    
    // Ensure HTTP response data is available
    if (!enhancedResult.response) {
      enhancedResult.response = '# HTTP response data not available from backend';
      console.warn("No HTTP response data found in exploit result");
    } else {
      console.log(`HTTP response data found: ${enhancedResult.response.length} characters`);
    }
    
    // Extract additional HTTP metadata if not explicitly provided by backend
    if (!enhancedResult.request_method && enhancedResult.request) {
      const methodMatch = enhancedResult.request.match(/^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\s+/);
      if (methodMatch) {
        enhancedResult.request_method = methodMatch[1];
        console.log(`Extracted HTTP method: ${enhancedResult.request_method}`);
      }
    }
    
    if (!enhancedResult.request_url && enhancedResult.request) {
      const urlMatch = enhancedResult.request.match(/^(?:GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\s+([^\s]+)/);
      if (urlMatch) {
        enhancedResult.request_url = urlMatch[1];
        console.log(`Extracted HTTP URL: ${enhancedResult.request_url}`);
      } else {
        // Try to find a Host header and construct a URL
        const hostMatch = enhancedResult.request.match(/Host:\s*([^\r\n]+)/i);
        if (hostMatch) {
          enhancedResult.request_url = `http://${hostMatch[1]}/`;
          console.log(`Constructed HTTP URL from Host header: ${enhancedResult.request_url}`);
        }
      }
    }
    
    // Extract status code from response if not explicitly provided
    if (!enhancedResult.status_code && enhancedResult.response) {
      const statusMatch = enhancedResult.response.match(/^HTTP\/[\d.]+\s+(\d+)/);
      if (statusMatch) {
        enhancedResult.status_code = parseInt(statusMatch[1], 10);
        console.log(`Extracted HTTP status code: ${enhancedResult.status_code}`);
      }
    }
    
    // Extract status text from response if not explicitly provided
    if (!enhancedResult.status_text && enhancedResult.response) {
      const statusTextMatch = enhancedResult.response.match(/^HTTP\/[\d.]+\s+\d+\s+(.+?)(?:\r\n|\n|\r)/);
      if (statusTextMatch) {
        enhancedResult.status_text = statusTextMatch[1];
        console.log(`Extracted HTTP status text: ${enhancedResult.status_text}`);
      }
    }
    
    console.log("Final enhanced exploit result:", enhancedResult);
    return enhancedResult;
  } catch (error) {
    console.error('Error exploiting vulnerability:', error);
    throw error;
  }
}

// Get common vulnerability types
export async function getCommonVulnerabilities(): Promise<string[]> {
  return invoke('get_common_vulnerabilities');
}

/**
 * Rhai Plugin Related APIs
 */

// Get all Rhai plugins
export async function getRhaiPlugins(): Promise<any[]> {
  return invoke('list_rhai_plugins');
}

// Get a specific Rhai plugin with its script content
export async function getRhaiPlugin(pluginId: string): Promise<any> {
  try {
    console.log(`API call: get_rhai_plugin with ID ${pluginId}`);
    const result = await invoke('get_rhai_plugin', { pluginId });
    console.log('API result:', result);
    return result;
  } catch (error) {
    console.error(`Error fetching Rhai plugin with ID ${pluginId}:`, error);
    throw error;
  }
}

// Upload a Rhai plugin
export async function uploadRhaiPlugin(file: File): Promise<any> {
  try {
    const contents = await file.text();
    return invoke('upload_rhai_plugin_content', { 
      filename: file.name,
      content: contents
    });
  } catch (error) {
    console.error('Error uploading Rhai plugin:', error);
    throw error;
  }
}

// Delete a Rhai plugin
export async function deleteRhaiPlugin(pluginName: string): Promise<boolean> {
  try {
    await invoke('delete_rhai_plugin', { pluginName });
    return true;
  } catch (error) {
    console.error('Error deleting Rhai plugin:', error);
    return false;
  }
}

// Update a Rhai plugin
export async function updateRhaiPlugin(params: {
  plugin_id: string;
  name: string;
  description: string;
  script: string;
}): Promise<any> {
  try {
    console.log(`API call: update_rhai_plugin with ID ${params.plugin_id}`);
    const result = await invoke('update_rhai_plugin', { 
      pluginId: params.plugin_id,
      name: params.name,
      description: params.description,
      script: params.script
    });
    console.log('Update result:', result);
    return result;
  } catch (error) {
    console.error(`Error updating Rhai plugin with ID ${params.plugin_id}:`, error);
    throw error;
  }
}

// Execute a Rhai plugin
export async function executeRhaiPlugin(params: {
  plugin_name: string;
  target: string;
  custom_params?: Record<string, any>;
  include_http_data?: boolean;
}): Promise<any> {
  try {
    console.log("Executing Rhai plugin with params:", JSON.stringify(params));
    
    const result = await invoke('execute_rhai_plugin', { 
      params: {
        plugin_name: params.plugin_name,
        plugin_type: params.plugin_name.split(':')[0],
        target: params.target,
        custom_params: params.custom_params || {},
        include_http_data: params.include_http_data !== false // Default to true
      }
    });
    
    // Process the result to ensure all expected fields are present
    const enhancedResult = result as ExploitResult;
    console.log("Raw plugin result:", JSON.stringify(enhancedResult));
    
    // Ensure HTTP request data is available
    if (!enhancedResult.request) {
      enhancedResult.request = '# HTTP request data not available from backend';
      console.warn("No HTTP request data found in plugin result");
    } else {
      console.log(`HTTP request data found: ${enhancedResult.request.length} characters`);
    }
    
    // Ensure HTTP response data is available
    if (!enhancedResult.response) {
      enhancedResult.response = '# HTTP response data not available from backend';
      console.warn("No HTTP response data found in plugin result");
    } else {
      console.log(`HTTP response data found: ${enhancedResult.response.length} characters`);
    }
    
    // Extract additional HTTP metadata if not explicitly provided by backend
    if (!enhancedResult.request_method && enhancedResult.request) {
      const methodMatch = enhancedResult.request.match(/^(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\s+/);
      if (methodMatch) {
        enhancedResult.request_method = methodMatch[1];
        console.log(`Extracted HTTP method: ${enhancedResult.request_method}`);
      }
    }
    
    if (!enhancedResult.request_url && enhancedResult.request) {
      const urlMatch = enhancedResult.request.match(/^(?:GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)\s+([^\s]+)/);
      if (urlMatch) {
        enhancedResult.request_url = urlMatch[1];
        console.log(`Extracted HTTP URL: ${enhancedResult.request_url}`);
      } else {
        // Try to find a Host header and construct a URL
        const hostMatch = enhancedResult.request.match(/Host:\s*([^\r\n]+)/i);
        if (hostMatch) {
          enhancedResult.request_url = `http://${hostMatch[1]}/`;
          console.log(`Constructed HTTP URL from Host header: ${enhancedResult.request_url}`);
        }
      }
    }
    
    // Extract status code from response if not explicitly provided
    if (!enhancedResult.status_code && enhancedResult.response) {
      const statusMatch = enhancedResult.response.match(/^HTTP\/[\d.]+\s+(\d+)/);
      if (statusMatch) {
        enhancedResult.status_code = parseInt(statusMatch[1], 10);
        console.log(`Extracted HTTP status code: ${enhancedResult.status_code}`);
      }
    }
    
    // Extract status text from response if not explicitly provided
    if (!enhancedResult.status_text && enhancedResult.response) {
      const statusTextMatch = enhancedResult.response.match(/^HTTP\/[\d.]+\s+\d+\s+(.+?)(?:\r\n|\n|\r)/);
      if (statusTextMatch) {
        enhancedResult.status_text = statusTextMatch[1];
        console.log(`Extracted HTTP status text: ${enhancedResult.status_text}`);
      }
    }
    
    console.log("Final enhanced exploit result:", enhancedResult);
    return enhancedResult;
  } catch (error) {
    console.error('Error executing Rhai plugin:', error);
    throw error;
  }
}

// Reload Rhai plugins from disk
export async function reloadRhaiPlugins(): Promise<boolean> {
  return invoke('load_rhai_plugins');
}

// Validate a plugin script
export async function validatePluginScript(script: string): Promise<boolean> {
  try {
    // Basic client-side validation to replace missing backend functionality
    if (!script || typeof script !== 'string') {
      throw new Error('Script must be a non-empty string');
    }

    // Check if script has the required functions - get_manifest and analyze
    if (!script.includes('fn get_manifest()') || !script.includes('fn analyze(')) {
      throw new Error('Script must contain both get_manifest() and analyze() functions');
    }
    
    // Check for basic script structure
    if (!script.includes('return') || !script.includes('let')) {
      throw new Error('Script appears to be missing basic language elements');
    }

    console.log('Client-side script validation passed');
    return true;
  } catch (error) {
    console.error('Error validating plugin script:', error);
    throw error;
  }
} 