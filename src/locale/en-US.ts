
/** simple end */
import localeSettings from './en-US/settings';
import ASMSettings from '@/views/asm/locale/en-US'
import SCANSettings from '@/views/scan/locale/en-US';
import VULNSettings from '@/views/vuln/locale/en-US';

import Plugin from '@/views/plugin/locale/en-US';


export default {
  'menu.asm': 'asm',
  'menu.vuln': 'vuln',
  'menu.scan': 'scan',
  'menu.brute': 'brute',
  'menu.setting': 'system',
  'menu.plugin': 'plugin',
  'navbar.action.locale': "english",



  ...localeSettings,
  ...ASMSettings,
  ...SCANSettings,
  ...VULNSettings,
  ...Plugin,
  /** simple end */
};
