/**
 * 漏洞插件状态
 */
export enum PluginStatus {
  DISABLED = 0,
  ENABLED = 1
}

/**
 * 漏洞插件类型
 */
export enum PluginType {
  RCE = 'rce',              // 远程代码执行
  DESERIALIZATION = 'deserialization', // 反序列化
  INJECTION = 'injection',  // 注入漏洞
  XSS = 'xss',              // 跨站脚本
  SSRF = 'ssrf',            // 服务端请求伪造
  FILE = 'file',            // 文件相关漏洞
  INFO = 'info',            // 信息泄露
  OTHER = 'other'           // 其他
}

/**
 * 漏洞严重性级别
 */
export enum SeverityLevel {
  CRITICAL = 'critical',
  HIGH = 'high',
  MEDIUM = 'medium',
  LOW = 'low',
  INFO = 'info'
}

/**
 * 插件参数类型
 */
export enum ParamType {
  STRING = 'string',
  NUMBER = 'number',
  BOOLEAN = 'boolean',
  SELECT = 'select',
  TEXTAREA = 'textarea',
  FILE = 'file'
}

/**
 * 插件参数定义
 */
export interface PluginParamDef {
  name: string;       // 参数名称
  key: string;        // 参数键
  type: ParamType;    // 参数类型
  required: boolean;  // 是否必须
  default?: any;      // 默认值
  description?: string; // 参数描述
  options?: Array<{label: string, value: any}>; // 当类型为select时的选项
}

/**
 * 漏洞插件定义
 */
export interface VulnPlugin {
  id?: number;           // 插件ID
  name: string;          // 插件名称
  rtype: PluginType;      // 插件类型
  description: string;   // 插件描述
  author: string;        // 作者
  version: string;       // 版本
  references: string[];  // 参考资料
  severity: SeverityLevel; // 严重程度
  createTime?: number;   // 创建时间
  updateTime?: number;   // 更新时间
  status?: PluginStatus;  // 插件状态
  script?: string;        // 插件脚本
  params: PluginParamDef[]; // 参数定义
  resultFields?: { // 结果字段定义
    name: string;      // 字段名称
    key: string;       // 字段键
    type: string;      // 字段类型
    description?: string; // 字段描述
  }[];
}

/**
 * 漏洞扫描结果
 */
export interface ScanResult {
  id?: number;                // 结果ID
  pluginId: number;           // 插件ID
  pluginName: string;         // 插件名称
  target: string;             // 目标
  timestamp: number;          // 时间戳
  status: 'success' | 'failed'; // 状态
  data: Record<string, any>;   // 结果数据
  details?: string;            // 详细信息
}

/**
 * 漏洞插件策略
 */
export interface PluginPolicy {
  id?: number;                // 策略ID
  name: string;               // 策略名称
  description: string;        // 策略描述
  plugins: number[];          // 插件ID列表
  createTime?: number;        // 创建时间
  updateTime?: number;        // 更新时间
}

/**
 * 扫描任务
 */
export interface ScanTask {
  id?: number;                // 任务ID
  name: string;               // 任务名称
  policyId: number;           // 策略ID
  policyName?: string;        // 策略名称
  targets: string[];          // 目标列表
  status: 'pending' | 'running' | 'completed' | 'failed'; // 状态
  progress?: number;          // 进度
  createTime?: number;        // 创建时间
  startTime?: number;         // 开始时间
  endTime?: number;           // 结束时间
  results?: ScanResult[];     // 结果
}

/**
 * 漏洞利用参数
 */
export interface ExploitParams {
  plugin_id?: number;
  target: string;
  custom_params?: Record<string, any>;
}

/**
 * 漏洞利用结果
 */
export interface ExploitResult {
  success: boolean;
  details: string;        // 详细信息
  data: Record<string, any>; // 结果数据
  raw_output: string;     // 原始输出
  
  // HTTP相关数据
  request?: string;        // HTTP请求内容（完整的原始HTTP请求）
  response?: string;       // HTTP响应内容（完整的原始HTTP响应）
  status_code?: number;    // HTTP状态码
  status_text?: string;    // HTTP状态描述
  request_method?: string; // HTTP请求方法（GET, POST等）
  request_url?: string;    // 请求的URL
  response_time?: number;  // 响应时间（毫秒）
  response_size?: number;  // 响应大小（字节）
  
  // 兼容旧接口
  message?: string;       // 消息（与details字段兼容）
} 