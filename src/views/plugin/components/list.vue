<template>
	<a-space direction="vertical" fill>
		<a-row justify="space-between">
			<a-col :span="12">
				<a-radio-group type="button" @change="onRDTypeChange" v-model:model-value="ptype">
					<a-radio value="all">{{ $t('plugin.all') }}</a-radio>
					<a-radio value="type">{{ $t('plugin.plugin_type') }}</a-radio>
				</a-radio-group>
			</a-col>
			<a-col :span="12">
				<a-row justify="end">
					<a-col flex="60px">
						<a-space>
							<!-- <a-button style="width: 95px;" type="primary" @click="onExport">{{ $t('asm.export')
								}}</a-button> -->
							<a-button style="width: 95px;" type="primary" @click="onAddPlugin">{{
								$t('plugin.add') }}</a-button>
						</a-space>
					</a-col>
				</a-row>
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="3">
				<a-select placeholder="filter" v-model="filterValue" size="small">
					<a-option value="name">插件名称</a-option>
					<a-option value="status">插件状态</a-option>
					<a-option value="plugin_type">插件类型</a-option>
				</a-select>
			</a-col>
			<a-col :span="21">
				<a-input-search placeholder="请输入待搜索的内容" @keyup.enter="RefreshData" v-model:model-value="search_key"
					@click="RefreshData" size="small" />
			</a-col>
		</a-row>

		<a-row>
			<a-col :span="24">

				<a-table :columns="all_columns" :bordered="false" :data="plugins.list" :pagination="pagination"
					:scroll="scroll" @page-change="onPageChange" size="small">

					<template #name="{ record }">
						<a-link @click="onViewPlugin(record)">{{ record.name }}</a-link>
					</template>


					<template #status="{ record }">
						<a-switch v-model="record.monitor_status" :checked-value="1" :unchecked-value="0"
							@change="onSwitchChange(record.id, record.monitor_status)" />
					</template>

					<template #time="{ record }">
						<template v-if="record.create_at != 0">创建: {{ formatDateTime(record.create_at) }}</template>
						<template v-else>创建时间:--</template>
						<br />
						<template v-if="record.update_at != 0">更新: {{ formatDateTime(record.update_at) }}</template>
						<template v-else>更新时间:--</template>
					</template>
					<template #count="{ record }">
						<a-link @click="toDomain(record.domain)">{{ record.count }}</a-link>
					</template>


					<template #operation="{ record }">
						<a-dropdown>
							<div class="clickable"><icon-more /></div>
							<template #content>
								<a-doption @click="onViewPlugin(record)">
									<template #icon>
										<icon-edit />
									</template>
									<template #default>{{ $t('plugin.modify') }}</template>
								</a-doption>
								<a-doption @click="onDelPlugin(record.id)">
									<template #icon>
										<icon-delete />
									</template>
									<template #default>{{ $t('plugin.delete') }}</template>
								</a-doption>
							</template>
						</a-dropdown>
					</template>
				</a-table>

			</a-col>
		</a-row>
	</a-space>
	<a-drawer :width="800" :header="false" :visible="add_visible" placement="right" @ok="handleOk"
		@cancel="handleCancel" unmountOnClose>
		<a-form :model="plugin" auto-label-width size="small" layout="horizontal" style="margin-top: 20px;">

			<a-form-item field="name" label="插件名称">
				<a-input v-model:model-value="plugin.name" :style="{ width: '200px' }" placeholder="插件名称" />
			</a-form-item>

			<a-form-item field="version" label="插件版本">
				<a-input v-model:model-value="plugin.version" :style="{ width: '200px' }" placeholder="插件版本" />

			</a-form-item>

			<a-form-item field="author" label="插件作者">
				<a-input v-model:model-value="plugin.author" :style="{ width: '200px' }" placeholder="插件作者" />
			</a-form-item>


			<a-form-item field="plugin_type" label="插件类型">
				<a-select :style="{ width: '200px' }" placeholder="插件类型" v-model:model-value="plugin.plugin_type"
					@search="handlerSearch" allow-search @change="handlerOpsChange">
					<template v-for="item in plugin_type" :key="item">
						<a-option :value="item">{{ item }}</a-option>
					</template>
				</a-select>
			</a-form-item>
			<a-form-item field="description" label="插件描述">
				<a-input :style="{ width: '400px' }" v-model:model-value="plugin.description" placeholder="插件描述" />
			</a-form-item>

			<a-form-item>
				<a-row :gutter="15">
					<a-col :span="12">
						<a-button @click="onTestScript(plugin.script)">验证插件</a-button>
					</a-col>
					<a-col :span="12"> 
						<a-button @click="onCopyExample">复制示例</a-button>
					</a-col>
				</a-row>
			</a-form-item>

			<a-form-item field="script">
				<codemirror v-model="plugin.script" :placeholder="example_script"
					:style="{ height: '500px', width: '100%', fontSize: '12px' }" :autofocus="true"
					:indent-with-tab="true" :tab-size="2" :extensions="extensions" @ready="handleReady" />
			</a-form-item>
		</a-form>
	</a-drawer>
</template>


<script setup lang="ts">
import { Pagination } from '@/types/global';
import { computed, onMounted, reactive, ref, shallowRef } from 'vue';
import { invoke } from "@tauri-apps/api/core";
import { Message } from '@arco-design/web-vue';
import { formatDateTime } from "@/utils/format"
import { Plugin, example_script } from './types';
import { useRouter } from 'vue-router'

import { Codemirror } from 'vue-codemirror'
import { javascript } from '@codemirror/lang-javascript'
import { oneDark } from '@codemirror/theme-one-dark'
import useClipboard from 'vue-clipboard3'


const { toClipboard } = useClipboard()
import { useI18n } from 'vue-i18n';
const { t } = useI18n();
const router = useRouter()

const pagination: Pagination = reactive({
	current: 1,
	total: 0,
	pageSize: 10,
	showTotal: true,
	showPageSize: true,
	showQuickJumper: true,
	showSizeChanger: true,
});

const extensions = [javascript(), oneDark]
const plugin_type = ref([]);
const plugins: { list: Plugin[] } = reactive({ list: [] });

const search_key = ref('')
const filterValue = ref('name')
const enterprise_id = ref('')
const add_visible = ref(false)
const ptype = ref('all')
const isEdit = ref(false)

const plugin = reactive({
	id: 0,
	name: '',
	version: '',
	description: '',
	author: '',
	plugin_type: '',
	input: '',
	output: '',
	status: 0,
	script: '',
	create_at: 0,
	update_at: 0,
})


const onPageChange = (_page: number) => {
	pagination.current = _page;

};

const onTestScript = async (script: String) => {
	await invoke("test_javascript", { script: script }).then((res: any) => {
		if (res) {
			Message.success(res)
		}
	}).catch((error) => {
		Message.warning(error)
	})
};


const onCopyExample = async () => {
	try {
		//item为要复制内容
		await toClipboard(example_script)
		//复制成功提示
		Message.success('复制成功')
	} catch (e) {
		//复制失败回调
		Message.warning('复制失败')
	}
}


const scroll = {
	y: 900
}
const view = shallowRef()

const handleReady = (payload: any) => {
	view.value = payload.view
}

async function RefreshData() {
	await invoke("get_plugin_list", { page: pagination.current, pagesize: pagination.pageSize, ptype: ptype.value, query: search_key.value }).then((res: any) => {
		if (res) {
			plugins.list = res.list
			pagination.total = res.total
		}
	}).catch((error) => console.error(error))
}




const onSwitchChange = async (pid: number, status: any) => {
	await invoke("switch_plugin_status", { pid: pid, status: status }).then((res: any) => {
		if (res) {
			Message.success("切换成功")
		}
	}).catch((err) => {
		Message.error("暂停失败，原因：" + err)
	})
};


const onViewPlugin = async (plg: Plugin) => {
	isEdit.value = true
	add_visible.value = true
	Object.assign(plugin, plg);
	await invoke("get_plugin_type_list").then((res: any) => {
		if (res) {
			plugin_type.value = res
		}
		RefreshData()
	});
};


const onRDTypeChange = async (value: string | number | boolean) => {
	ptype.value = value as string
	switch (value) {
		case 'all':
			let res: any = await invoke("get_plugin_list", { page: pagination.current, pagesize: pagination.pageSize });
			if (res) {
				plugins.list = res
			}
			break
	}
}

onMounted(() => {
	RefreshData()
})

const onDelPlugin = async (pid: string) => {
	await invoke("del_plugins_by_id", { pid: pid }).then((res: any) => {
		RefreshData()
		Message.info(res);
	});
}


const handlerOpsChange = (value: string | number | boolean | Record<string, any> | (string | number | boolean | Record<string, any>)[]) => {
	enterprise_id.value = value as string
}


const handlerSearch = async () => {
	await invoke("get_enterprise_list").then((res: any) => {
		if (res) {
			plugins.list = res.list;
		}
	});

}




const toDomain = async (domain: string) => {
	router.push({
		name: 'asm-domain', query: { domain: domain }
	})
}

const onAddPlugin = async () => {
	add_visible.value = true

	await invoke("get_plugin_type_list").then((res: any) => {
		if (res) {
			plugin_type.value = res
		}
		RefreshData()
	});


}

async function handleOk() {
	add_visible.value = false

	if (isEdit) {
		await invoke("save_plugin", { plugin }).then((res: any) => {
			if (res) {
				Message.success(res)
			}
			RefreshData()
		}).catch((err: any) => Message.error(err));
	} else {
		await invoke("new_plugin", { plugin }).then((res: any) => {
			if (res) {
				Message.success(res)
			}
			RefreshData()
		}).catch((err: any) => Message.error(err));
	}

	isEdit.value = false
}

function handleCancel() {
	add_visible.value = false
}

const all_columns = computed(() => {
	return [
		{
			title: 'ID',
			dataIndex: 'id',
		},
		{
			title: t('plugin.name'),
			dataIndex: 'name',
			slotName: 'name',

		},
		{
			title: t('plugin.status'),
			dataIndex: 'status',
			slotName: 'status',
		},
		{
			title: t('plugin.plugin_type'),
			dataIndex: 'plugin_type',
			slotName: 'plugin_type',
		},
		{
			title: t('plugin.time'),
			dataIndex: 'time',
			slotName: 'time',
			width: 210
		},
		{
			title: t('asm.operation'),
			slotName: "operation",
		},
	];

});



</script>


<style lang="less" scoped></style>