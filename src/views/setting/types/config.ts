export interface AsmConfig {
  dns_collection_brute_status: boolean;
  dns_collection_plugin_status: boolean;
  is_buildin: boolean;
  proxy?: string;
  user_agent?: string;
  http_headers?: Array<[string, string]>;
  http_timeout?: number;
  thread_num?: number;
  file_dict?: string;
  subdomain_dict?: string;
  subdomain_level?: number;
}

export interface HeaderItem {
  key: string;
  value: string;
} 