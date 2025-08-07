import { invoke } from '@tauri-apps/api/core';

// 定义扫描器状态接口
export interface ScannerStatus {
  running: boolean;
  proxy_address: string;
  proxy_port: number;
  scan_count: number;
  vulnerability_count: number;
  message?: string;  // 状态信息
  last_update?: string;  // 最后更新时间
  last_stop_time?: string;  // 最后停止时间
}

// 定义漏洞详情接口
export interface VulnerabilityDetail {
  note:string
  request: string;
  response: string;
}

// 定义漏洞接口
export interface Vulnerability {
  id: number;
  vulnerability_type: string;
  name: string;
  url: string;
  risk_level: string; // "Critical", "High", "Medium", "Low", "Info"
  timestamp: string;
  description: string;
  solution: string;
  parameter?: string;         // 漏洞参数名
  value?: string;             // 参数值或payload
  evidence?: string;          // 漏洞证据
  request_details?: string;   // 构造的请求详情
  response_details?: string;  // 响应详情
  details?: VulnerabilityDetail;
}

// 原始扫描器配置，用于被动扫描表单
export interface ScannerConfig {
  port: number;
  intercept_tls: boolean;
  save_results: boolean;
  results_path: string;
}

// 被动扫描API所需的新配置接口
export interface ScanConfig {
  port: number;
  target_url: string;
  depth: number;
  max_pages: number;
  intercept_tls: boolean;  // 是否拦截TLS
  save_results: boolean;   // 是否保存结果
  results_path: string;    // 结果保存路径
  use_plugins: boolean;  // 是否使用插件
}

export interface ActiveScanConfig {
  targets: string[];
  scan_type: string;
  threads: number;
  timeout: number;
  save_results: boolean;
  results_path?: string;
  detailed_scan_options?: {
    host_survival?: boolean; // 主机存活扫描
    port_scan?: { // 端口扫描配置
      enabled?: boolean;
      ports?: string; // 例如 "1-1000", "80,443,22" 或 "top100"
    };
    vulnerability_scan?: { // 漏洞扫描配置
      enabled?: boolean;
      plugins?: string[]; // 例如 ["xss", "sqli"] 或 "all"
    };
    web_sensitive_info?: boolean; // Web敏感信息扫描
    service_bruteforce?: { // 服务爆破配置
      enabled?: boolean;
      services?: string[]; // 例如 ["ssh", "ftp"]
      usernames?: string; // 用户名列表文件路径或逗号分隔列表
      passwords?: string; // 密码列表文件路径或逗号分隔列表
    };
    fingerprint_scan?: boolean; // 指纹识别
    nuclei_scan?: boolean; // Nuclei 扫描 (如果想把它作为详细选项之一)
    vulnerability_exploit?: { // 漏洞利用配置
      enabled?: boolean;
      options?: string[]; // 例如 ["ssh_pubkey", "cron_job", "remote_command", "ms17_010"]
    };
  };
}

export interface SuccessResponse {
  success: boolean;
  message: string;
}

export interface AssetStatistics {
  total_domains: number;
  total_ips: number;
  total_ports: number;
  total_websites: number;
  total_vulnerabilities: number;
  risk_distribution: {
    critical: number;
    high: number;
    medium: number;
    low: number;
    info: number;
  };
}

// Scanner API service
const scannerService = {
  // Start the passive scanner with configuration
  async startPassiveScanner(config: ScanConfig): Promise<boolean> {
    try {
      // 确保所有配置字段都被发送到后端
      const scanConfig = {
        port: config.port,
        target_url: config.target_url,
        depth: config.depth || 2,
        max_pages: config.max_pages || 100,
        intercept_tls: config.intercept_tls || false,
        save_results: config.save_results || false,
        results_path: config.results_path || '',
        use_plugins: config.use_plugins || false
      };
      
      await invoke('start_passive_scan', { config: scanConfig });
      return true;
    } catch (e) {
      console.error('Failed to start passive scanner:', e);
      throw e;
    }
  },

  // Start the active scanner with configuration
  async startActiveScan(config: ActiveScanConfig): Promise<boolean> {
    try {
      const response = await invoke<SuccessResponse>('start_active_scan', { config });
      return response.success;
    } catch (error) {
      console.error('Failed to start active scanner:', error);
      return false;
    }
  },

  // Stop the scanner
  async stopScanner(): Promise<boolean> {
    try {
      await invoke('stop_passive_scan');
      return true;
    } catch (e) {
      console.error('Failed to stop scanner:', e);
      throw e;
    }
  },

  // Get scanner status
  async getStatus(): Promise<ScannerStatus> {
    try {
      return await invoke('get_scan_status');
    } catch (e) {
      console.error('Failed to get scanner status:', e);
      throw e;
    }
  },

  // Get vulnerabilities
  async getVulnerabilities(limit?: number): Promise<Vulnerability[]> {
    try {
      // 调用后端API获取漏洞列表
      const vulnerabilities = await invoke<Vulnerability[]>('get_scan_vulnerabilities');
      
      // 如果指定了limit，则限制返回数量
      if (limit && vulnerabilities.length > limit) {
        return vulnerabilities.slice(0, limit);
      }
      
      return vulnerabilities;
    } catch (e) {
      console.error('获取漏洞列表失败:', e);
      throw e;
    }
  },

  // Get vulnerability by ID
  async getVulnerabilityById(id: number): Promise<Vulnerability | null> {
    try {
      const response = await invoke<Vulnerability>('get_vulnerability_by_id', { id });
      return response;
    } catch (error) {
      console.error(`Failed to fetch vulnerability with ID ${id}:`, error);
      return null;
    }
  },

  // Clear all vulnerabilities
  async clearVulnerabilities(): Promise<boolean> {
    try {
      const response = await invoke<SuccessResponse>('clear_scan_vulnerabilities');
      return response.success;
    } catch (e) {
      console.error('清除漏洞列表失败:', e);
      throw e;
    }
  },

  // Export vulnerabilities to file
  async exportVulnerabilities(_format: string, path?: string): Promise<boolean> {
    try {
      // 确保有导出路径
      if (!path) {
        console.error('导出路径不能为空');
        return false;
      }
      
      const response = await invoke<SuccessResponse>('export_scan_vulnerabilities', { 
        path 
      });
      return response.success;
    } catch (e) {
      console.error('导出漏洞列表失败:', e);
      throw e;
    }
  },

  // 获取资产统计数据
  async getAssetStatistics(taskId: number): Promise<AssetStatistics> {
    try {
      return await invoke<AssetStatistics>('get_asset_statistics', { taskId });
    } catch (error) {
      console.error('Failed to fetch asset statistics:', error);
      return {
        total_domains: 0,
        total_ips: 0,
        total_ports: 0,
        total_websites: 0,
        total_vulnerabilities: 0,
        risk_distribution: {
          critical: 0,
          high: 0,
          medium: 0,
          low: 0,
          info: 0
        }
      };
    }
  },

  // 手动触发端口扫描
  async triggerPortScan(_taskId: number): Promise<boolean> {
    try {
      // const response = await invoke<SuccessResponse>('port_scan', { taskId: taskId });
      return true;
    } catch (error) {
      console.error('Failed to trigger port scan:', error);
      return false;
    }
  },
};

export default scannerService;