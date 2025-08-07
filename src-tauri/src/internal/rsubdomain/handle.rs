use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::Duration,
};

use log::error;
use pnet::packet::{
    dns::{DnsPacket, DnsTypes},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    udp::UdpPacket,
    Packet,
};

use super::model::DnsRecord;
use super::{
    send,
    structs::{LOCAL_STACK, LOCAL_STATUS},
};

pub fn handle_dns_packet(
    rst_send: mpsc::SyncSender<DnsRecord>,
    dns_recv: mpsc::Receiver<Arc<Vec<u8>>>,
    flag_id: u16,
    running: Arc<AtomicBool>,
) {
    while running.load(Ordering::Relaxed) {
        match dns_recv.recv_timeout(Duration::from_millis(100)) {
            Ok(ipv4_packet) => {
                if let Some(ipv4) = Ipv4Packet::new(ipv4_packet.as_ref()) {
                    if ipv4.get_next_level_protocol() == IpNextHeaderProtocols::Udp {
                        if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                            if let Some(dns) = DnsPacket::new(udp.payload()) {
                                if dns.get_is_response() == 0 {
                                    continue;
                                }
                                let tid = dns.get_id() / 100;
                                if tid == flag_id {
                                    if dns.get_response_count() > 0 {
                                        let query_name = dns.get_queries()[0].get_qname_parsed();

                                        for res in dns.get_responses() {
                                            match res.rtype {
                                                DnsTypes::A => {
                                                    let ip = res
                                                        .data
                                                        .iter()
                                                        .map(|byte| byte.to_string())
                                                        .collect::<Vec<String>>()
                                                        .join(".");
                                                    let dns_record = DnsRecord {
                                                        domain: query_name.clone(),
                                                        record_type: "A".to_string(),
                                                        record_value: ip,
                                                    };
                                                    match rst_send.send(dns_record) {
                                                        Ok(_) => (),
                                                        Err(_) => error!("send rst_result failed"),
                                                    }
                                                }
                                                DnsTypes::CNAME => {
                                                    let mut cname = String::new();
                                                    let mut i = 0;
                                                    while i < res.data.len() {
                                                        let len = res.data[i] as usize;
                                                        if len == 0 {
                                                            break;
                                                        }
                                                        if i + 1 + len <= res.data.len() {
                                                            let label =
                                                                &res.data[i + 1..i + 1 + len];
                                                            if !cname.is_empty() {
                                                                cname.push('.');
                                                            }
                                                            cname.push_str(
                                                                &String::from_utf8_lossy(label),
                                                            );
                                                            i += 1 + len;
                                                        } else {
                                                            break;
                                                        }
                                                    }
                                                    // println!("res.data: {:?}", res.data);
                                                    let dns_record = DnsRecord {
                                                        domain: query_name.clone(),
                                                        record_type: "CNAME".to_string(),
                                                        record_value: cname,
                                                    };
                                                    match rst_send.send(dns_record) {
                                                        Ok(_) => (),
                                                        Err(_) => error!("send rst_result failed"),
                                                    }
                                                }
                                                DnsTypes::NS => {
                                                    let ns = res
                                                        .data
                                                        .iter()
                                                        .filter(|&&byte| byte != 0)
                                                        .map(|&byte| byte as char)
                                                        .collect::<String>();
                                                    let dns_record = DnsRecord {
                                                        domain: query_name.clone(),
                                                        record_type: "NS".to_string(),
                                                        record_value: ns,
                                                    };
                                                    match rst_send.send(dns_record) {
                                                        Ok(_) => (),
                                                        Err(_) => error!("send rst_result failed"),
                                                    }
                                                }
                                                DnsTypes::MX => {
                                                    let mx = res
                                                        .data
                                                        .iter()
                                                        .filter(|&&byte| byte != 0)
                                                        .map(|&byte| byte as char)
                                                        .collect::<String>();
                                                    let dns_record = DnsRecord {
                                                        domain: query_name.clone(),
                                                        record_type: "MX".to_string(),
                                                        record_value: mx,
                                                    };
                                                    match rst_send.send(dns_record) {
                                                        Ok(_) => (),
                                                        Err(_) => error!("send rst_result failed"),
                                                    }
                                                }
                                                DnsTypes::TXT => {
                                                    let txt = res
                                                        .data
                                                        .iter()
                                                        .filter(|&&byte| byte != 0)
                                                        .map(|&byte| byte as char)
                                                        .collect::<String>();
                                                    let dns_record = DnsRecord {
                                                        domain: query_name.clone(),
                                                        record_type: "TXT".to_string(),
                                                        record_value: txt,
                                                    };
                                                    match rst_send.send(dns_record) {
                                                        Ok(_) => (),
                                                        Err(_) => error!("send rst_result failed"),
                                                    }
                                                }
                                                _ => (),
                                            }
                                        }
                                    }
                                    match LOCAL_STATUS.write() {
                                        Ok(mut local_status) => {
                                            let index = send::generate_map_index(
                                                dns.get_id() % 100,
                                                udp.get_destination(),
                                            );
                                            match local_status
                                                .search_from_index_and_delete(index as u32)
                                            {
                                                Ok(_data) => {
                                                    // println!("[+] delete recv:{:?}", data.v);
                                                }
                                                Err(_) => (),
                                            }

                                            match LOCAL_STACK.try_write() {
                                                Ok(mut stack) => {
                                                    if stack.length <= 50000 {
                                                        stack.push(index as usize)
                                                    }
                                                }
                                                Err(_) => (),
                                            }
                                        }
                                        Err(_) => (),
                                    };
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => (),
        }
    }
    drop(rst_send);
    // println!("recv dns packet done");
}

// let tid = dns.get_id() / 100;
// if tid == flag_id {
//     if dns.get_response_count() > 0 {
//         for query in dns.get_queries() {
//             query_name.push_str(
//                 query.get_qname_parsed().as_str(),
//             )
//         }
//         for res in dns.get_responses() {
//             match res.rtype{
//                 DnsTypes::A =>{
//                     println!(
//                         "{} =>  {}",
//                         query_name, res.data.iter().map(|byte| byte.to_string()).collect::<Vec<String>>().join(".")
//                     );
//                 },
//                 DnsTypes::CNAME =>{

//                     println!(
//                         "{} => CNAME {:?}",
//                         query_name, res.data
//                     );
//                 }
//                 _ =>()
//             }
//         }
//     }
// match LOCAL_STATUS.try_write() {
//     Ok(mut local_status) => {
//         let index = send::generate_map_index(
//             dns.get_id() % 100,
//             udp.get_destination(),
//         );
//         match local_status
//             .search_from_index_and_delete(index as u32)
//         {
//             Ok(data) => {
//                 // println!("delete:{:?}", data.v);
//                 count += 1;
//             }
//             Err(_) => (),
//         }

//         if count/50 == 0{
//             println!("delete:{}", count);

//         }

//         match LOCAL_STACK.try_write() {
//             Ok(mut stack) => {
//                 if stack.length <= 50000 {
//                     stack.push(index as usize)
//                 }
//             }
//             Err(_) => (),
//         }

//         if dns.get_response_count() > 0 {
//             for query in dns.get_queries() {
//                 if query.get_qname_parsed() == "mail.mgtv.com"{
//                     println!("{} ", query.get_qname_parsed());
//                 }
//                 query_name.push_str(
//                     query.get_qname_parsed().as_str(),
//                 )
//             }
//             for res in dns.get_responses() {
//                 if res.rtype == DnsTypes::A {
//                     println!(
//                         "{} -> {:?}",
//                         query_name, res.data
//                     );
//                 }
//             }
//         }
//     }
//     Err(_) => (),
// };

// }
