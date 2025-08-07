use pnet::transport::{transport_channel, TransportChannelType, TransportProtocol, TransportSender, TransportReceiver};
use pnet::packet::ipv4::{MutableIpv4Packet, Ipv4Flags, Ipv4Packet};
use pnet::packet::udp::{MutableUdpPacket, UdpPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::{Packet, MutablePacket};
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};
use rand::Rng;

const TEST_DURATION: Duration = Duration::from_secs(10);
const PAYLOAD_SIZE: usize = 1000; // UDP payload size in bytes
const IP_HEADER_SIZE: usize = 20; // Assuming no IP options
const UDP_HEADER_SIZE: usize = 8;
// Total length of the IP packet to be sent/received
const EXPECTED_PACKET_LEN: usize = IP_HEADER_SIZE + UDP_HEADER_SIZE + PAYLOAD_SIZE;


#[derive(Debug)]
pub struct SpeedTestResult {
    pub max_upload_mbps: f64,
    pub max_download_mbps: f64,
    pub max_pps_sent: u64,
}

fn get_source_ip() -> Option<Ipv4Addr> {
    for iface in pnet::datalink::interfaces() {
        for ip_network in iface.ips {
            if let pnet::ipnetwork::IpNetwork::V4(ipv4_network) = ip_network {
                let ip = ipv4_network.ip();
                if !ip.is_loopback() && !ip.is_link_local() && !ip.is_multicast() && ip.octets()[0] != 0 {
                    // Basic check for a usable public or private unicast IP
                    return Some(ip);
                }
            }
        }
    }
    None
}

pub fn perform_speed_test(target_ip_str: &str, target_port: u16) -> Result<SpeedTestResult, String> {
    let target_ipv4: Ipv4Addr = target_ip_str.parse().map_err(|e| format!("无效的目标IP地址: {}", e))?;
    
    let source_ipv4 = get_source_ip().ok_or_else(|| "无法确定合适的源IP地址。请检查网络接口配置。".to_string())?;
    
    let mut rng = rand::thread_rng();
    let source_port: u16 = rng.gen_range(49152..=65535); // Ephemeral port range

    println!("源 IP: {}, 源端口: {}", source_ipv4, source_port);
    println!("目标 IP: {}, 目标端口: {}", target_ipv4, target_port);

    let protocol = TransportChannelType::Layer4(TransportProtocol::Ipv4(IpNextHeaderProtocols::Udp));
    // Buffer needs to be large enough for multiple packets
    let (mut tx, mut rx) = transport_channel(EXPECTED_PACKET_LEN * 512, protocol)
        .map_err(|e| format!("创建传输通道时出错: {}. 可能需要管理员权限。", e))?;

    // --- 上传测试 ---
    let mut total_payload_bytes_sent = 0u64;
    let mut total_packets_sent = 0u64;
    let mut pps_this_second_upload = 0u64;
    let mut max_pps_sent = 0u64;

    let mut upload_payload_data = vec![0u8; PAYLOAD_SIZE];
    rng.fill(&mut upload_payload_data[..]);

    println!("开始上传测试到 {}:{}，持续 {:?}...", target_ipv4, target_port, TEST_DURATION);
    let upload_start_time = Instant::now();
    let mut last_pps_calc_time_upload = Instant::now();

    let mut packet_id_counter: u16 = 0;

    while Instant::now().duration_since(upload_start_time) < TEST_DURATION {
        let mut ip_packet_buffer = vec![0u8; EXPECTED_PACKET_LEN];
        
        packet_id_counter = packet_id_counter.wrapping_add(1);

        // 构建IP和UDP包
        {
            let mut ip_packet = MutableIpv4Packet::new(&mut ip_packet_buffer).unwrap();
            ip_packet.set_version(4);
            ip_packet.set_header_length(5); // 5 * 4 bytes = 20 bytes
            ip_packet.set_dscp(0);
            ip_packet.set_ecn(0);
            ip_packet.set_total_length(EXPECTED_PACKET_LEN as u16);
            ip_packet.set_identification(packet_id_counter); // Simple packet ID
            ip_packet.set_flags(Ipv4Flags::DontFragment);
            ip_packet.set_fragment_offset(0);
            ip_packet.set_ttl(64);
            ip_packet.set_next_level_protocol(IpNextHeaderProtocols::Udp);
            ip_packet.set_source(source_ipv4);
            ip_packet.set_destination(target_ipv4);

            let mut udp_packet = MutableUdpPacket::new(ip_packet.payload_mut()).unwrap();
            udp_packet.set_source(source_port);
            udp_packet.set_destination(target_port);
            udp_packet.set_length((UDP_HEADER_SIZE + PAYLOAD_SIZE) as u16);
            udp_packet.set_payload(&upload_payload_data);
            
            let udp_checksum = pnet::packet::udp::ipv4_checksum(&udp_packet.to_immutable(), &source_ipv4, &target_ipv4);
            udp_packet.set_checksum(udp_checksum);
            
            // IP checksum should be calculated last, after header and payload are set
            let ip_checksum = pnet::packet::ipv4::checksum(&ip_packet.to_immutable());
            ip_packet.set_checksum(ip_checksum);
        }
        
        // Ensure the packet we are sending is valid before trying to create an immutable view
        let final_packet_to_send = Ipv4Packet::new(&ip_packet_buffer).ok_or_else(|| "创建待发送的Ipv4Packet失败".to_string())?;

        match tx.send_to(final_packet_to_send, IpAddr::V4(target_ipv4)) {
            Ok(size_passed_to_os) => {
                if size_passed_to_os > 0 { // size_passed_to_os is the size of the IP packet
                    total_payload_bytes_sent += PAYLOAD_SIZE as u64;
                    total_packets_sent += 1;
                    pps_this_second_upload += 1;
                } else {
                    // Possibly non-blocking send and buffer is full, or other issue
                    std::thread::sleep(Duration::from_micros(50)); // Brief pause
                }
            }
            Err(_e) => {
                // eprintln!("上传发送错误: {}", e); // Can be noisy
                std::thread::sleep(Duration::from_micros(100)); // Brief pause on error
            }
        }

        if Instant::now().duration_since(last_pps_calc_time_upload) >= Duration::from_secs(1) {
            if pps_this_second_upload > max_pps_sent {
                max_pps_sent = pps_this_second_upload;
            }
            pps_this_second_upload = 0;
            last_pps_calc_time_upload = Instant::now();
        }
    }

    let upload_duration_actual = Instant::now().duration_since(upload_start_time);
    let upload_mbps = if upload_duration_actual.as_secs_f64() > 0.0 {
        (total_payload_bytes_sent * 8) as f64 / (upload_duration_actual.as_secs_f64() * 1024.0 * 1024.0)
    } else { 0.0 };

    if pps_this_second_upload > max_pps_sent { // 捕获最后一个不足一秒的区间的PPS
        max_pps_sent = pps_this_second_upload;
    }
    println!("上传测试完成。速率: {:.2} Mbps, 最大发送 PPS: {}", upload_mbps, max_pps_sent);

    // --- 下载测试 ---
    // 此测试依赖于目标服务器将UDP数据包发送回 source_ipv4:source_port
    println!("开始下载测试，从 {}:{} 监听，持续 {:?}...", target_ipv4, target_port, TEST_DURATION);
    let mut total_payload_bytes_received = 0u64;
    let download_start_time = Instant::now();
    
    // Create a new iterator for each test phase if necessary, or ensure rx is properly managed
    let mut rx_iter = pnet::transport::ipv4_packet_iter(&mut rx);

    while Instant::now().duration_since(download_start_time) < TEST_DURATION {
        match rx_iter.next_with_timeout(Duration::from_millis(100)) {
            Ok(Some((packet, addr))) => {
                if addr == IpAddr::V4(target_ipv4) && packet.get_next_level_protocol() == IpNextHeaderProtocols::Udp {
                    if let Some(udp_packet) = UdpPacket::new(packet.payload()) {
                        if udp_packet.get_destination_port() == source_port { // 确保是发给我们的包
                            total_payload_bytes_received += udp_packet.payload().len() as u64;
                        }
                    }
                }
            }
            Ok(None) => { /* Timeout, no packet received */ }
            Err(e) => {
                eprintln!("下载接收错误: {}", e);
                break; 
            }
        }
    }
    let download_duration_actual = Instant::now().duration_since(download_start_time);
    let download_mbps = if download_duration_actual.as_secs_f64() > 0.0 {
        (total_payload_bytes_received * 8) as f64 / (download_duration_actual.as_secs_f64() * 1024.0 * 1024.0)
    } else { 0.0 };
    println!("下载测试完成。速率: {:.2} Mbps", download_mbps);

    Ok(SpeedTestResult {
        max_upload_mbps: upload_mbps,
        max_download_mbps: download_mbps,
        max_pps_sent,
    })
}

// Example usage (you'd call this from elsewhere, e.g. a Tauri command)
// fn main() {
//     // Replace with a known UDP echo server or your test server IP and port
//     // For example, a public echo server might work for basic tests, but behavior can vary.
//     // Cloudflare: 1.1.1.1 (does not typically run a general purpose UDP echo on arbitrary ports)
//     // It's best to use a server you control for reliable testing.
//     match perform_speed_test("TARGET_SERVER_IP", 7) { // Port 7 is often echo protocol
//         Ok(results) => {
//             println!("Speed Test Results: {:?}", results);
//         }
//         Err(e) => {
//             eprintln!("Speed Test Failed: {}", e);
//         }
//     }
// }
