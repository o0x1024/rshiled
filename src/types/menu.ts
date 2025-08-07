const menuList: Array<any> = [
  {
    path: '/asm',
    name: 'asm',
    meta: {
      locale: 'menu.asm',
      order: 1,
      keepAlive: true,
    }
  },
  {
    path: '/vuln',
    name: 'vuln',
    meta: {
      locale: 'menu.vuln',
      order: 2,
      keepAlive: true,
    },
  },
  {
    path: '/scan',
    name: 'scan',
    meta: {
      locale: 'menu.scan',
      order: 3,
    },
  },
  {
    path: '/proxy',
    name: 'proxy',
    meta: {
      locale: 'menu.proxy',
      order: 4,
    },
  },
  {
    path: '/brute',
    name: 'brute',
    meta: {
      locale: 'menu.brute',
      order: 5,
    },
  },
  {
    path: '/setting',
    name: 'setting',
    meta: {
      locale: 'menu.setting',
      order: 6,
    },
  },
];

export { menuList };