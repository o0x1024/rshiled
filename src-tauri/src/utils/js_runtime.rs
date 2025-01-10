use deno_core::error::AnyError;
use deno_core::{extension, serde, serde_v8, v8, PollEventLoopOptions};
use deno_core::{op2, serde_json};
use reqwest::header::{ HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::rc::Rc;
use std::vec;

use crate::plugin::plugin::Plugin;




#[derive(Serialize, Deserialize, Debug)]
struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

#[op2(async)]
#[string]
async fn op_read_file(#[string] path: String) -> Result<String, AnyError> {
    let contents = tokio::fs::read_to_string(path).await?;
    Ok(contents)
}

#[op2(async)]
async fn op_write_file(#[string] path: String, #[string] contents: String) -> Result<(), AnyError> {
    tokio::fs::write(path, contents).await?;
    Ok(())
}

#[op2(fast)]
fn op_remove_file(#[string] path: String) -> Result<(), AnyError> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[op2(async)]
#[serde]
async fn op_fetch_post(
    #[string] url: String,
    #[string] data: String,
    #[string] proxy: String,
    #[serde] headers: Vec<(String, String)>,
) -> Result<HttpResponse, AnyError> {
    // println!("url:{}", url);
    // println!("proxy:{}", proxy);
    // println!("header:{:?}", headers);
    // println!("data:{:?}", data);
    let client: Client;
    if proxy.is_empty() {
        client = reqwest::Client::new();
    } else {
        let proxy = reqwest::Proxy::all(proxy)?;
        client = reqwest::Client::builder().proxy(proxy).build()?;
    }

    let mut req_headers = HeaderMap::new();
    if headers.len() > 0 {
        for (key, value) in headers {
            req_headers.insert(
                HeaderName::from_bytes(key.as_bytes())?,
                HeaderValue::from_str(&value).unwrap(),
            );
        }
    }

    let response = if let Ok(json_data) = serde_json::from_str::<serde_json::Value>(data.as_str()) {
        // 如果解析成功，发送 JSON 数据
        client
            .post(url)
            .json(&json_data)
            .headers(req_headers)
            .send()
            .await?
    } else {
        // 如果解析失败，发送普通字符串
        client
            .post(url)
            .body(data)
            .headers(req_headers)
            .send()
            .await?
    };

    let status = response.status().as_u16();

    // 获取响应的头部
    let headers = response
        .headers()
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    // 获取响应的文本内容
    let body = response.text().await?;

    // let body = client.post(url).await?.text().await?;
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

#[op2(async)]
#[serde]
async fn op_fetch_get(
    #[string] url: String,
    #[string] proxy: String,
    #[serde] headers: Vec<(String, String)>,
) -> Result<HttpResponse, AnyError> {
    println!("url:{}", url);
    println!("proxy:{}", proxy);
    println!("header:{:?}", headers);

    let client: Client;
    if proxy.is_empty() {
        client = reqwest::Client::new();
    } else {
        let proxy = reqwest::Proxy::all(proxy)?;
        client = reqwest::Client::builder().proxy(proxy).build()?;
    }

    // println!("header:{:?}",headers);

    let mut req_headers = HeaderMap::new();
    for (key, value) in headers {
        req_headers.insert(
            HeaderName::from_bytes(key.as_bytes())?,
            HeaderValue::from_str(&value).unwrap(),
        );
    }
    let response = client.get(url).headers(req_headers).send().await?;
    let status = response.status().as_u16();

    // 获取响应的头部
    let headers = response
        .headers()
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_str().unwrap_or("").to_string()))
        .collect();

    // 获取响应的文本内容
    let body = response.text().await?;

    // 返回封装的 HttpResponse
    Ok(HttpResponse {
        status,
        headers,
        body,
    })

    // let http_response = HttpResponse {
    //     // status,
    //     headers,
    //     body,
    // };
    // Ok(json!(http_response))
}

#[op2(fast)]
fn op_push_msg(#[string] message: String) -> Result<(), AnyError> {
    Ok(())
}

extension!(runjs, ops = [op_read_file, op_write_file, op_remove_file,op_fetch_get,op_fetch_post,op_push_msg,],
  esm_entry_point = "ext:runjs/runtime.js",
  esm = [dir "src", "runtime.js"],);

async fn execute_script(code: &str, params: serde_json::Value) -> Result<Value, AnyError> {
    // let main_module = deno_core::resolve_path("./utils.js", &std::env::current_dir()?)?;
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![runjs::init_ops_and_esm()],
        ..Default::default()
    });

    // 将参数注入到全局作用域
    let params_str = serde_json::to_string(&params)?;
    js_runtime.execute_script("<params>", format!("globalThis.params = {};", params_str))?;

    let res = js_runtime.execute_script("<anon>", code.to_string());
    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await?;

    match res {
        Ok(global) => {
            let scope = &mut js_runtime.handle_scope();
            let local = v8::Local::new(scope, global);

            // 尝试将返回值转换为 Promise
            if let Ok(promise) = v8::Local::<v8::Promise>::try_from(local) {
                // 尝试将返回值转换为 Promise
                let promise_result = promise.result(scope);

                // 将解析结果转换为 serde_json::Value
                let value = serde_v8::from_v8::<Value>(scope, promise_result)?;
                Ok(value)
            } else {
                // 如果返回值不是 Promise，直接转换为 serde_json::Value
                let value = serde_v8::from_v8::<Value>(scope, local)?;
                Ok(value)
            }
        }
        Err(err) => Err(err.into()),
    }
}

pub async fn dns_collection_by_plugin(domain: &str) -> Result<Vec<String>, String> {
    let mut result_domains = Vec::new();

    // Get database connection
    let db_path = crate::utils::file::get_db_path();
    let conn = Connection::open(db_path).unwrap();

    // Query enabled DNS collection plugins
    let mut stmt = conn
        .prepare("SELECT * FROM plugins WHERE status = 1 AND plugin_type = 'dns_collection'")
        .unwrap();

    let plugin_iter = stmt
        .query_map([], |row| {
            Ok(Plugin {
                id: row.get(0)?,
                name: row.get(1)?,
                version: row.get(2)?,
                description: row.get(3)?,
                author: row.get(4)?,
                plugin_type: row.get(5)?,
                input: row.get(6)?,
                output: row.get(7)?,
                status: row.get::<_, isize>(8)?,
                script: row.get(9)?,
                create_at: row.get(10)?,
                update_at: row.get(11)?,
            })
        })
        .unwrap();

    // Execute each plugin
    for plugin in plugin_iter {
        if let Ok(plugin) = plugin {
            // Create params for the plugin
            let params = serde_json::json!({
                "domain": domain
            });

            // Execute plugin script
            match execute_script(&plugin.script, params).await {
                Ok(value) => {
                    match value {
                        Value::Array(arr) => {
                            println!("Script returned an array:");
                            for (index, item) in arr.iter().enumerate() {
                                println!("  [{}] = {:?}", index, item);
                            }
                        }
                        _ => {
                            // 处理其他类型
                            println!("Script returned an unexpected type");
                        }
                    }
                }
                Err(err) => {
                    println!("excute_script error:{}", err)
                }
            }
        }
    }

    Ok(result_domains)
}

fn Run() {
    let params = serde_json::json!({
        "domain": "mgtv.com",
    });

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

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        match execute_script(&xcode, params).await {
            Ok(value) => {
                match value {
                    Value::Array(arr) => {
                        println!("Script returned an array:");
                        for (index, item) in arr.iter().enumerate() {
                            println!("  [{}] = {:?}", index, item.as_str());
                        }
                    }
                    _ => {
                        // 处理其他类型
                        println!("Script returned an unexpected type");
                    }
                }
            }
            Err(err) => {
                println!("excute_script error:{}", err)
            }
        }
    })

    // 运行事件循环以处理异步操作
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run() {
        Run()
    }
}
