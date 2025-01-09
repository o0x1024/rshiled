import { createApp } from "vue";
import i18n from './locale';
import App from "./App.vue";
import ArcoVue from '@arco-design/web-vue';
import store from './store';
import router from './router';
import ArcoVueIcon from '@arco-design/web-vue/es/icon';
import '@arco-design/web-vue/dist/arco.css';


import '@/assets/style/global.less';

const app = createApp(App);
app.use(i18n);
app.use(router);
app.use(store);
app.use(ArcoVue, {});
app.use(ArcoVueIcon);
app.mount('#app');



