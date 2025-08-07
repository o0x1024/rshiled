import { invoke } from '@tauri-apps/api/core';

// 协议类型
export enum Protocol {
  SSH = 'SSH',
  SMB = 'SMB',
  RDP = 'RDP',
  MySQL = 'MySQL',
  MSSQL = 'MSSQL',
  Redis = 'Redis',
  PostgreSQL = 'PostgreSQL',
  Oracle = 'Oracle',
  FTP = 'FTP',
  Telnet = 'Telnet',
}

// 任务状态
export enum TaskStatus {
  Pending = 'Pending',
  Running = 'Running',
  Completed = 'Completed',
  Failed = 'Failed',
  Stopped = 'Stopped',
}

// 暴力破解任务
export interface BruteForceTask {
  id?: number;
  name: string;
  target: string;
  port: number;
  protocol: Protocol;
  username_file?: string;
  password_file?: string;
  usernames?: string[];
  passwords?: string[];
  threads: number;
  timeout: number;
  created_at?: number;
  status: TaskStatus;
}

// 暴力破解结果
export interface BruteForceResult {
  task_id: number;
  target: string;
  protocol: Protocol;
  username: string;
  password: string;
  success: boolean;
  time_taken: number;
  error?: string;
}

// 创建新任务
export async function createTask(task: BruteForceTask): Promise<number> {
  return invoke('brute_create_task', { task });
}

// 获取所有任务
export async function getTasks(): Promise<BruteForceTask[]> {
  return invoke('brute_get_tasks');
}

// 获取任务结果
export async function getResults(taskId: number): Promise<BruteForceResult[]> {
  return invoke('brute_get_results', { taskId });
}

// 删除任务
export async function deleteTask(taskId: number): Promise<boolean> {
  return invoke('brute_delete_task', { taskId });
}

// 启动任务
export async function startTask(taskId: number): Promise<void> {
  return invoke('brute_start_task', { taskId });
}

// 停止任务
export async function stopTask(taskId: number): Promise<void> {
  return invoke('brute_stop_task', { taskId });
} 