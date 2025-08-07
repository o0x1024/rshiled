use std::{collections::HashMap, future::Future, pin::Pin};

pub mod certificates;
pub mod datasets;
// use tokio;

type AsyncFn =
    Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = Vec<String>> + Send>> + Send + Sync>;

fn get_dns_collection_func() -> Result<HashMap<String, AsyncFn>, String> {
    let mut func_map: HashMap<String, AsyncFn> = HashMap::new();

    // 直接插入函数指针
    func_map.insert(
        "ip138".to_string(),
        Box::new(|domain: String| {
            let future = async move { datasets::ip138::get_ip138_subdomains(domain).await };
            Box::pin(future) as Pin<Box<dyn Future<Output = Vec<String>> + Send + Sync>>
        }),
    );

    // func_map.insert(
    //     "certspotter".to_string(),
    //     Box::new(|domain: String| {
    //         let future = async move { get_certspotter_subdomains(domain).await };
    //         Box::pin(future) as Pin<Box<dyn Future<Output = Vec<String>> + Send + Sync>>
    //     }),
    // );

    Ok(func_map)
}

pub async fn dns_collection_by_api(domain: &str) -> Result<Vec<String>, String> {
    let mut domains = Vec::new();
    match get_dns_collection_func() {
        Ok(func_map) => {
            for (_, mfunc) in func_map {
                let ds = mfunc(domain.to_string()).await; // 调用 func_a
                domains.extend(ds);
            }
        }
        Err(e) => println!("Error: {}", e),
    }
    Ok(domains)
}

#[cfg(test)]

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dns_collection_by_api() {
        let base_url = "mgtv.com";
        dns_collection_by_api(base_url).await.unwrap();
    }
}
