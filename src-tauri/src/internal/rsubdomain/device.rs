use log::{debug, error, info};
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet::EthernetPacket;
use pnet::packet::ipv6::Ipv6Packet;
use pnet::{
    datalink,
    packet::{
        dns::DnsPacket, ethernet::EtherTypes, ip::IpNextHeaderProtocols, ipv4::Ipv4Packet,
        udp::UdpPacket, Packet,
    },
};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::iter::repeat_with;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{net::IpAddr, process::Command};
use tokio::sync::mpsc;

use super::model::EthTable;

pub async fn auto_get_devices(running: &Arc<AtomicBool>) -> Result<EthTable, String> {
    let interfaces = datalink::interfaces();

    let (mptx, mut mprx) = mpsc::channel::<EthTable>(1);
    let domain = random_str(4) + ".example.com";

    info!("test domain:{}", domain);
    for interface in interfaces {
        if !interface.is_loopback() {
            for ip in interface.ips.clone() {
                match ip.ip() {
                    IpAddr::V4(_) => {
                        // println!("interface:{}", &interface);

                        let domain_clone = domain.clone();
                        let interface_clone = interface.clone();
                        let interface_name = interface.name.clone();
                        let mptx_clone = mptx.clone();
                        let running_clone = Arc::clone(running);
                        tokio::spawn(async move {
                            let (_, mut rx) =
                                match datalink::channel(&interface_clone, Default::default()) {
                                    Ok(Ethernet(_tx, _rx)) => (_tx, _rx),
                                    Ok(_) => {
                                        error!("不支持的通道类型");
                                        return;
                                    }
                                    Err(e) => {
                                        error!("创建数据链路通道时出错: {}", e);
                                        return;
                                    }
                                };
                            while running_clone.load(Ordering::Relaxed) {
                                match rx.next() {
                                    Ok(packet) => {
                                        let ethernet = EthernetPacket::new(packet).unwrap();
                                        match ethernet.get_ethertype() {
                                            EtherTypes::Ipv4 => {
                                                let ipv4_packet =
                                                    Ipv4Packet::new(ethernet.payload()).unwrap();
                                                match ipv4_packet.get_next_level_protocol() {
                                                    IpNextHeaderProtocols::Udp => {
                                                        let udp_packet =
                                                            UdpPacket::new(ipv4_packet.payload())
                                                                .unwrap();

                                                        if udp_packet.get_source() != 53 {
                                                            continue;
                                                        }

                                                        if let Some(dns) =
                                                            DnsPacket::new(udp_packet.payload())
                                                        {
                                                            for query in dns.get_queries() {
                                                                let recv_domain =
                                                                    query.get_qname_parsed();
                                                                // println!("recv_domain:{}", recv_domain);
                                                                if recv_domain.eq(&domain_clone) {
                                                                    let ipv4 = match ip.ip() {
                                                                        IpAddr::V4(addr) => addr,
                                                                        IpAddr::V6(_) => {
                                                                            error!("Expected an IPv4 address, got an IPv6 address");
                                                                            return;
                                                                        }
                                                                    };
                                                                    if let Err(err) = mptx_clone
                                                                        .send(EthTable {
                                                                            src_ip: ipv4,
                                                                            device: interface_name,
                                                                            src_mac: ethernet
                                                                                .get_destination(),
                                                                            dst_mac: ethernet
                                                                                .get_source(),
                                                                        })
                                                                        .await
                                                                    {
                                                                        error!("An error occurred when sending the message: {}", err);
                                                                    }
                                                                    return;
                                                                }
                                                            }
                                                        }
                                                    }
                                                    _ => (),
                                                }
                                            }
                                            EtherTypes::Ipv6 => {
                                                let ipv6_packet =
                                                    Ipv6Packet::new(ethernet.payload());
                                                if let Some(header) = ipv6_packet {
                                                    match header.get_next_header() {
                                                        IpNextHeaderProtocols::Udp => {
                                                            let udp_packet =
                                                                UdpPacket::new(header.payload())
                                                                    .unwrap();

                                                            if udp_packet.get_source() != 53 {
                                                                continue;
                                                            }

                                                            if let Some(dns) =
                                                                DnsPacket::new(udp_packet.payload())
                                                            {
                                                                for query in dns.get_queries() {
                                                                    let recv_domain =
                                                                        query.get_qname_parsed();
                                                                    if recv_domain
                                                                        .contains(&domain_clone)
                                                                    {
                                                                        println!("auto_get_device get domain:{}",recv_domain);
                                                                        let ipv4 = match ip.ip() {
                                                                            IpAddr::V4(addr) => {
                                                                                addr
                                                                            }
                                                                            IpAddr::V6(_) => {
                                                                                error!("Expected an IPv4 address, got an IPv6 address");
                                                                                return;
                                                                            }
                                                                        };
                                                                        if let Err(err) = mptx_clone
                                                                        .send(EthTable {
                                                                            src_ip: ipv4,
                                                                            device: interface_name,
                                                                            src_mac: ethernet
                                                                                .get_destination(),
                                                                            dst_mac: ethernet
                                                                                .get_source(),
                                                                        }).await
                                                                    {
                                                                        debug!("An error occurred when sending the message: {}", err);
                                                                    }
                                                                        return;
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        _ => (),
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "An error occurred when reading from the datalink channel: {}",
                                            e
                                        );
                                        continue;
                                    }
                                }
                            }
                        });
                    }
                    _ => (),
                }
            }
        }
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    Command::new("ping")
        .arg("-c")
        .arg("1")
        .arg(domain)
        .output()
        .unwrap();

    match mprx.recv().await {
        Some(eth) => Ok(eth),
        None => {
            error!("recv error: channel closed");
            return Err("recv error: channel closed".to_string());
        }
    }
}

fn random_str(n: usize) -> String {
    let mut rng = thread_rng();
    // 生成一个长度为 n 的随机字符串
    repeat_with(|| rng.sample(Alphanumeric) as char)
        .take(n)
        .collect()
}
