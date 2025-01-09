
/** simple end */
import localeSettings from './zh-CN/settings';
import ASMSettings from '@/views/asm/locale/zh-CN';
import SCANSettings from '@/views/scan/locale/zh-CN';
import VULNSettings from '@/views/vuln/locale/zh-CN';
import Plugin from '@/views/plugin/locale/zh-CN';


export default {
  'menu.asm': '攻击面',
  'menu.vuln': '漏洞相关',
  'menu.scan': '扫描相关',
  'menu.brute': '暴破相关',
  'menu.setting': '系统设置',
  'menu.plugin': '插件配置',
  'navbar.action.locale': "中文",

  ...localeSettings,
  ...ASMSettings,
  ...SCANSettings,
  ...VULNSettings,
  ...Plugin,

  /** simple end */
};
