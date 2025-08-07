/** simple end */
import localeSettings from './zh-CN/settings';
import ASMSettings from '@/views/asm/locale/zh-CN';
import SCANSettings from '@/views/scan/locale/zh-CN';
import VULNSettings from '@/views/vuln/locale/zh-CN';
import Setting from '@/views/setting/locale/zh-CN';
import PROXYSettings from '@/views/proxy/locale/zh-CN';
import REPEATERSettings from '@/components/repeater/locale/zh-CN';
import BRUTESettings from '@/views/brute/locale/zh-CN';

export default {
  'menu.asm': '攻击面',
  'menu.vuln': '漏洞模块',
  'menu.scan': '扫描模块',
  'menu.scan.passive': '被动扫描',
  'menu.scan.active': '主动扫描',
  'menu.proxy': '代理模块',
  'menu.brute': '暴破相关',
  'menu.setting': '系统设置',
  'menu.plugin': '插件配置',
  'menu.plugins': '插件',
  'menu.plugins.management': '插件管理',
  'navbar.action.locale': "中文",
  'menu.repeater': '重放模块',


  ...Setting,
  ...localeSettings,
  ...ASMSettings,
  ...SCANSettings,
  ...VULNSettings,
  ...REPEATERSettings,
  ...BRUTESettings,
  ...PROXYSettings,
  'asm.dashboard.overview': '攻击面概览',
  'asm.dashboard.total_domains': '域名总数',
  'asm.dashboard.total_ips': 'IP总数',
  'asm.dashboard.total_ports': '端口总数',
  'asm.dashboard.total_websites': '网站总数',
  'asm.dashboard.risk_distribution': '风险分布',
  'asm.dashboard.quick_actions': '快捷操作',
  'asm.dashboard.start_scan': '开始扫描',
  'asm.dashboard.refresh_data': '刷新数据',
  'asm.dashboard.export_report': '导出报告',
  'asm.dashboard.recent_findings': '最新发现',

  'setting.asm.enabled': '启用',
  'setting.asm.disabled': '禁用',
  'setting.asm.basicSettings': '基本设置',
  'setting.asm.dns_collection_brute': '子域名暴力破解',
  'setting.asm.dns_collection_plugin': '插件识别域名',
  'setting.asm.thread_num': '扫描线程数',
  'setting.asm.http_timeout': 'HTTP请求超时时间 (秒)',
  'setting.asm.proxy': 'HTTP代理',
  'setting.asm.user_agent': 'User-Agent',
  'setting.asm.http_headers': 'HTTP 头',
  'setting.asm.add_header': '添加 HTTP 头',
  'setting.asm.save': '保存配置',
  'setting.asm.reset': '重置',
  'setting.asm.save_success': '配置已更新',
  'setting.asm.save_error': '更新配置失败',
  'setting.asm.load_error': '加载配置失败',

  settings: {
    navbar: {
      log: '查看日志',
    },
    log: {
      title: '日志查看器',
      path: '日志文件路径',
      refresh: '刷新日志',
    },
  },

  'proxy.title': '代理设置',

  /** simple end */
};
