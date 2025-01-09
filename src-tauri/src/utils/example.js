
let code = r#"
async function getexample(){
console.log('gelengelen');
    const headers = [
        ['Content-Type', 'application/json'],
        ['Authorization', 'Bearer token'],
        ['Accept', 'application/json']
    ];
console.log('gelengelen1');
    const res = await plugin.fetch_get('https://example.com','123456',headers);
    // plugin.push_msg(res.body)
    console.log('gelengelen2');
    return res.body
}

    (async () => {
     return await getexample();
    })();
 "#;

let xcode = r#"
async function fetch(){
    console.log('Parameters:', params);
    const regex = /(?:https?:\/\/)?(?:www\.)?([a-zA-Z0-9-]+\.[a-zA-Z]{2,6}(?:\.[a-zA-Z]{2,6})?)/g;
    const text = "请访问我们的站点 https://www.example.com 或者 http://subdomain.example.co.uk 了解更多信息。";
    const matches = text.match(regex);
    console.log('gelengelen');
    return matches;
}
(async () => {
     let res = await fetch();
     return res;
})();
"#;


let scode = r#"
(async () => { return 'abc'})()
"#;





// ... existing code ...

async fn run_js(code: &str, params: serde_json::Value) -> Result<(), AnyError> {
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![runjs::init_ops_and_esm()],
        ..Default::default()
    });

    // 将参数注入到全局作用域
    let params_str = serde_json::to_string(&params)?;
    js_runtime.execute_script(
        "<params>",
        format!("globalThis.params = {};", params_str),
    )?;

    // 执行主代码
    let res = js_runtime.execute_script("<anon>", code.to_string());
    // ... existing code ...
}

fn Run() {
    // 示例：传递参数给 JavaScript
    let params = serde_json::json!({
        "url": "https://example.com",
        "token": "your-token",
        "headers": {
            "Content-Type": "application/json"
        }
    });

    let code = r#"
    async function getexample(){
        console.log('Parameters:', params);  // 可以直接访问 params
        const headers = [
            ['Content-Type', params.headers['Content-Type']],
            ['Authorization', `Bearer ${params.token}`],
            ['Accept', 'application/json']
        ];
        const res = await plugin.fetch_get(params.url, '123456', headers);
        return res.body
    }

    (async () => {
         return await getexample();
    })();
    "#;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    if let Err(error) = runtime.block_on(run_js(code, params)) {
        eprintln!("error: {}", error);
    }
}



let postcode = r#"
async function search() {
    const headers = [
    ['Content-Type', 'application/json'],
    ['Authorization', 'Bearer token'],
    ['Accept', 'application/json']
    ];
    const proxyUrl = "http://127.0.0.1:8080";
    let page_num = 0;
    let per_page_num = 50;
    let result_domains = [];
    const regex = /(?:https?:\/\/)?(?:www\.)?([a-zA-Z0-9-]+\.[a-zA-Z]{2,6}(?:\.[a-zA-Z]{2,6})?)/g;

    let limit_num = 20;
    while (true) {
        const query = 'site:.' + params.domain;
        const param = `q=${query}&first=${page_num}&count=${per_page_num}`;
        console.log(param);
        const res = await plugin.fetch_post('https://www.bing.com/search',param,proxyUrl,headers);
        let text = res.body;

        const subdomains = text.match(regex);
        if(subdomains.lenght > 0){
            result_domains.concat(subdomains)
        }
        // if (!this.check_subdomains(subdomains)) break;
        if (!text.includes('<div class="sw_next">')) break;
        page_num += per_page_num;
        if (page_num >=limit_num) break;
    }
    console.log(result_domains);
    return result_domains;  
}
(async () => {
     return await search();
})();
 "#;