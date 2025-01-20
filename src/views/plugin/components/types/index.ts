export interface Plugin {
	id: number
	// 插件的名称
	name: string
	// 插件的版本号
	version: string
	// 插件的描述信息
	description: string
	// 插件的作者
	author: string
	// 插件的类型，用于区分不同类型的插件
	plugin_type: string
	// 插件的输入参数或输入数据
	input: string
	// 插件的输出结果或输出数据
	output: string
	// 插件的状态，表示插件是启用还是禁用
	status: number
	// 插件的脚本代码，通常用于定义插件的具体行为
	script: string
	create_at: number
	update_at: number
}




// export const  example_script = "async function search() { \
//         const headers = [\
//             ['Content-Type', 'application/json'],\
//             ['Authorization', 'Bearer token'],\
//             ['Accept', 'application/json']];\
//         const proxyUrl = 'http://127.0.0.1:8080';\
//         let result_domains = [];\
//         const regex = new RegExp(`(?:>|"|'|=|,)(?:http://|https://)?(?:[a-z0-9](?:[a-z0-9\\-]{0,61}[a-z0-9])?\\.)*${params.domain}`, 'g');\
//         const uu =`https://site.ip138.com/${params.domain}/domain.htm`\
//         const res = await plugin.fetch_get(uu,proxyUrl,headers);\
//         let text = res.body;\
//         const subdomains = text.match(regex);\
//         if(subdomains != null && subdomains.length  > 0){\
//             result_domains = result_domains.concat(subdomains);\
//         }\
//         return result_domains;\
//     }\
//     (async () => {\
//          return await search();\
//     })();";



export const example_script = `async function search() { 
    const headers = [
        ['Content-Type', 'application/json'],
        ['Authorization', 'Bearer token'],
        ['Accept', 'application/json']];
    const proxyUrl = 'http://127.0.0.1:8080';
    let result_domains = [];

    const regex = new RegExp("(?:>|\\"|'|=|,)(?:http://|https://)?(?:[a-z0-9](?:[a-z0-9\-]{0,61}[a-z0-9])?\.)*"+params.domain, 'gi');
    const uu ='https://site.ip138.com/' + params.domain +'/domain.htm'
    const res = await plugin.fetch_get(uu,proxyUrl,headers);
    let text = res.body;
    const subdomains = text.match(regex);
    console.log(subdomains)
    if(subdomains != null && subdomains.length  > 0){
        subdomains.forEach(subdomain => {
            subdomain = subdomain.replace(/^[>"'=,]+/, '');
            subdomain = subdomain.replace(/^https?:\\/\\//, '');
            subdomain = subdomain.toLowerCase();
            result_domains = result_domains.concat(subdomain);
        });
    }
    return result_domains;
}
(async () => {
    return await search();
})();`;


