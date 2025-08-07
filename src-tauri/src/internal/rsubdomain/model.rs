use pnet::datalink::MacAddr;
use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct EthTable {
    pub src_ip: Ipv4Addr,
    pub device: String,
    pub src_mac: MacAddr,
    pub dst_mac: MacAddr,
}

#[derive(Clone, Debug)]
pub struct StatusTable {
    pub domain: String,    // 查询域名
    pub dns: String,       // 查询dns
    pub time: u64,         // 发送时间
    pub retry: i32,        // 重试次数
    pub domain_level: i32, // 域名层级
}

#[derive(Clone, Debug)]
pub struct DnsRecord {
    pub domain: String,       // 域名
    pub record_type: String,  // 记录类型 (A, CNAME, NS, MX, TXT)
    pub record_value: String, // 记录值
}
