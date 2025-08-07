use log::{debug, info};
use rand;
use rand::Rng;
use sqlx::query_scalar;

use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, RwLock};
use std::thread::sleep;
use std::time::Duration;

use crate::internal::rsubdomain::send::SendDog;
use crate::internal::rsubdomain::structs::{RetryStruct, LOCAL_STATUS};
use crate::internal::rsubdomain::{device, handle, recv, send, subdata};

/// 检查应用是否在root权限下运行
pub fn is_running_as_root() -> bool {
    #[cfg(target_family = "unix")]
    {
        // Unix系统 (Linux, macOS)下检查
        return match Command::new("id").arg("-u").output() {
            Ok(output) => {
                let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
                uid == "0"
            }
            Err(_) => false,
        };
    }

    #[cfg(target_family = "windows")]
    {
        // Windows系统下检查管理员权限
        return match Command::new("net").args(["session"]).output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        };
    }
}
use super::model::DnsRecord;

pub async fn domain_brute_by_rsubdomain(
    root_domains: &Vec<String>,
    domains: &Vec<String>,
    level: usize,
    is_buildin: bool,
    dict_path: String,
) -> Result<Vec<DnsRecord>, ()> {
    let (rst_send, rst_recv) = mpsc::sync_channel::<DnsRecord>(100);

    let running = Arc::new(AtomicBool::new(true));
    let ether = {
        let running_clone = Arc::clone(&running);
        device::auto_get_devices(&running_clone).await.unwrap()
    };

    info!("ether:{:?}", ether);
    let mut rng: rand::prelude::ThreadRng = rand::thread_rng();
    let flag_id: u16 = rng.gen_range(400..655);
    let device_clone = ether.device.clone();

    let senddog = Arc::new(Mutex::new(SendDog::new(ether, vec![], flag_id)));
    let sub_domain_list: Box<Vec<String>>;

    let (dns_send, dns_recv) = mpsc::channel();
    let (retry_send, retry_recv): (
        mpsc::Sender<Arc<RwLock<RetryStruct>>>,
        mpsc::Receiver<Arc<RwLock<RetryStruct>>>,
    ) = mpsc::channel();
    {
        //网卡收包
        let running_clone = Arc::clone(&running);
        tokio::spawn(async move {
            recv::recv(device_clone, dns_send, &running_clone);
        });
    }

    {
        let running_clone: Arc<AtomicBool> = Arc::clone(&running);
        //处理收到的包
        tokio::spawn(async move {
            handle::handle_dns_packet(rst_send, dns_recv, flag_id, running_clone);
        });
    }

    //根据is_buildin 判断是否使用内置字典
    if is_buildin {
        sub_domain_list = subdata::get_default_sub_next_data();
    } else {
        sub_domain_list = subdata::get_sub_next_data(dict_path);
    }

    for sub in sub_domain_list.iter() {
        for rt in root_domains {
            let mut senddog = senddog.try_lock().unwrap();
            let mut final_domain = sub.clone();
            final_domain.push_str(".");
            final_domain.push_str(&rt);
            let dns_name = senddog.chose_dns();
            let (flagid2, scr_port) =
                senddog.build_status_table(final_domain.as_str(), dns_name.as_str(), 1);
            senddog.send(final_domain, dns_name, scr_port, flagid2);
        }
    }

    // 先进行异步操作获取子域名列表
    // 从配置中获取子域名级别，默认为3

    let sub_domain_list_result = generate_subdomain_wordlist(root_domains, domains, level);

    // println!("{:?}",sub_domain_list_result);

    for sub in sub_domain_list_result.iter() {
        let mut senddog = senddog.try_lock().unwrap();
        let dns_name = senddog.chose_dns();
        let (flagid2, scr_port) = senddog.build_status_table(sub.as_str(), dns_name.as_str(), 1);
        senddog.send(sub.clone(), dns_name, scr_port, flagid2);
    }

    let senddog_clone: Arc<Mutex<SendDog>> = Arc::clone(&senddog);
    {
        let running: Arc<AtomicBool> = running.clone();
        //处理超时的域名
        tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                let mut is_delay = true;
                let mut datas = Vec::new();
                match LOCAL_STATUS.write() {
                    Ok(mut local_status) => {
                        let max_length = (1000000 / 10) as usize;
                        datas = local_status.get_timeout_data(max_length);
                        is_delay = datas.len() > 100;
                    }
                    Err(_) => (),
                }

                for local_data in datas {
                    let index = local_data.index;
                    let mut value = local_data.v;

                    if value.retry >= 5 {
                        // 处理失败的索引
                        match LOCAL_STATUS.write() {
                            Ok(mut local_status) => {
                                match local_status.search_from_index_and_delete(index as u32) {
                                    Ok(_data) => {
                                        // println!("main delete:{:?}", data.v);
                                    }
                                    Err(_) => (),
                                }
                                continue;
                            }
                            Err(_) => (),
                        }
                    }
                    let senddog = senddog_clone.lock().unwrap();
                    value.retry += 1;
                    value.time = chrono::Utc::now().timestamp() as u64;
                    value.dns = senddog.chose_dns(); // 假设有一个选择 DNS 的函数
                    let value_c = value.clone();
                    {
                        match LOCAL_STATUS.write() {
                            Ok(mut local_status) => {
                                let _ = local_status.search_from_index_and_delete(index);
                                local_status.append(value_c, index);
                            }
                            Err(_) => {}
                        }
                    }

                    let (flag_id, src_port) = send::generate_flag_index_from_map(index as usize); // 假设有这个函数
                    let retry_struct = RetryStruct {
                        domain: value.domain,
                        dns: value.dns,
                        src_port,
                        flag_id,
                        domain_level: value.domain_level as usize,
                    };
                    //发送重试结构体到通道
                    let _ = match retry_send.send(Arc::new(RwLock::new(retry_struct))) {
                        Ok(_) => (),
                        Err(_err) => {
                            // error!("重试通道发送失败:{}", err);
                        }
                    };

                    if is_delay {
                        let sleep_duration = rand::thread_rng().gen_range(100..=400);
                        sleep(Duration::from_micros(sleep_duration));
                    }
                }
            }
        });
    }

    {
        let running_clone: Arc<AtomicBool> = Arc::clone(&running);
        let senddog = Arc::clone(&senddog);
        //超时的重发
        tokio::spawn(async move {
            while running_clone.load(Ordering::Relaxed) {
                match retry_recv.recv() {
                    Ok(res) => {
                        let rety_data = res.read().unwrap();
                        let senddog = senddog.lock().unwrap();

                        senddog.send(
                            rety_data.domain.clone(),
                            rety_data.dns.clone(),
                            rety_data.src_port,
                            rety_data.flag_id,
                        )
                    }
                    Err(_) => (),
                }
            }
        });
    }

    let mut rst_result_list = Vec::<DnsRecord>::new();
    let timeout_duration = Duration::from_millis(100); // 设置100ms超时

    // 设置一个最大空闲时间，如果超过这个时间没有收到结果，则认为已经完成
    let mut idle_timeout_count = 0;
    let max_idle_count = 150; // 10秒后无数据则退出

    while running.load(Ordering::Relaxed) {
        match rst_recv.recv_timeout(timeout_duration) {
            Ok(rst_result) => {
                // println!("收到数据: {:?}", rst_result);
                rst_result_list.push(rst_result);
                idle_timeout_count = 0; // 收到数据，重置空闲计数器
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // 超时，增加空闲计数
                idle_timeout_count += 1;

                // 检查是否应该继续运行
                if !running.load(Ordering::Relaxed) || idle_timeout_count >= max_idle_count {
                    info!("超过{}秒未收到新数据，结束处理", max_idle_count / 10);
                    break;
                }
                continue;
            }
            Err(_) => {
                break;
            }
        }
    }

    running.store(false, Ordering::Relaxed);
    Ok(rst_result_list)
}

pub fn generate_subdomain_wordlist(
    target_domains: &Vec<String>,
    domains: &Vec<String>,
    level: usize,
) -> Vec<String> {
    // 存储不同级别子域名的HashMap
    let mut subdomain_levels: HashMap<usize, HashSet<String>> = HashMap::new();

    // 处理每个域名，提取不同级别的子域名
    for domain in domains {
        let parts: Vec<&str> = domain.split('.').collect();

        // 只处理至少有2个部分的域名（至少是二级域名）
        if parts.len() >= 2 {
            // 提取各级别的子域名部分
            for i in 0..parts.len() - 1 {
                let level = i + 1; // 级别从1开始计数
                let part = parts[i];

                // 过滤无效子域名部分
                if !part.is_empty() && part != "www" {
                    subdomain_levels
                        .entry(level)
                        .or_insert_with(HashSet::new)
                        .insert(part.to_string());
                }
            }
        }
    }

    let mut result = Vec::new();

    //target_domains 是rootdomain
    for target_domain in target_domains {
        // 解析目标域名
        let target_parts: Vec<&str> = target_domain.split('.').collect();
        if target_parts.len() < 2 {
            continue; // 跳过无效域名，而不是返回错误
        }

        // 基础域名（通常是二级域名，如example.com）
        let base_domain = if target_parts.len() == 2 {
            target_domain.to_string()
        } else {
            target_parts[target_parts.len() - 2..].join(".")
        };

        match level {
            3 => {
                // 暴破三级域名
                if let Some(prefixes) = subdomain_levels.get(&1) {
                    for prefix in prefixes {
                        result.push(format!("{}.{}", prefix, base_domain));
                    }
                }
            }
            4 => {
                // 暴破四级域名，先生成所有可能的三级域名
                if let Some(l3_prefixes) = subdomain_levels.get(&1) {
                    let third_level_domains: Vec<String> = l3_prefixes
                        .iter()
                        .map(|prefix| format!("{}.{}", prefix, base_domain))
                        .collect();

                    // 然后为每个三级域名添加四级前缀
                    if let Some(l4_prefixes) = subdomain_levels.get(&2) {
                        for third_domain in &third_level_domains {
                            for prefix in l4_prefixes {
                                result.push(format!("{}.{}", prefix, third_domain));
                            }
                        }
                    }
                }
            }
            5 => {
                // 暴破五级域名
                // 先生成所有可能的三级域名
                if let Some(l3_prefixes) = subdomain_levels.get(&1) {
                    let third_level_domains: Vec<String> = l3_prefixes
                        .iter()
                        .map(|prefix| format!("{}.{}", prefix, base_domain))
                        .collect();

                    // 然后生成所有可能的四级域名
                    if let Some(l4_prefixes) = subdomain_levels.get(&2) {
                        let fourth_level_domains: Vec<String> = third_level_domains
                            .iter()
                            .flat_map(|third_domain| {
                                l4_prefixes
                                    .iter()
                                    .map(|prefix| format!("{}.{}", prefix, third_domain))
                                    .collect::<Vec<String>>()
                            })
                            .collect();

                        // 最后为每个四级域名添加五级前缀
                        if let Some(l5_prefixes) = subdomain_levels.get(&3) {
                            for fourth_domain in &fourth_level_domains {
                                for prefix in l5_prefixes {
                                    result.push(format!("{}.{}", prefix, fourth_domain));
                                }
                            }
                        }
                    }
                }
            }
            _ => {
                return vec![];
            }
        }
    }

    result
}
