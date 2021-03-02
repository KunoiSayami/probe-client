/*
 ** Copyright (C) 2021 KunoiSayami
 **
 ** This file is part of probe-client and is released under
 ** the AGPL v3 License: https://www.gnu.org/licenses/agpl-3.0.txt
 **
 ** This program is free software: you can redistribute it and/or modify
 ** it under the terms of the GNU Affero General Public License as published by
 ** the Free Software Foundation, either version 3 of the License, or
 ** any later version.
 **
 ** This program is distributed in the hope that it will be useful,
 ** but WITHOUT ANY WARRANTY; without even the implied warranty of
 ** MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 ** GNU Affero General Public License for more details.
 **
 ** You should have received a copy of the GNU Affero General Public License
 ** along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::thread;
use std::time::Duration;
use systemstat::{saturating_sub_bytes, LoadAverage, Platform, System, Filesystem, NetworkStats, Memory, CPULoad};
use log::{error};
use std::fmt::Formatter;

#[derive(Serialize, Deserialize)]
struct MountInfo {
    mount_from: String,
    mount_type: String,
    mount_on: String,
    mount_avail: u64,
    mount_total: u64,
}

impl From<&systemstat::Filesystem> for MountInfo {
    fn from(fs: &Filesystem) -> Self {
        let mount_from = fs.fs_mounted_from.clone();
        let mount_type = fs.fs_type.clone();
        let mount_on = fs.fs_mounted_on.clone();
        let mount_avail = fs.avail;
        let mount_total = fs.total;
        MountInfo {mount_from, mount_type, mount_on, mount_avail: mount_avail.as_u64(), mount_total: mount_total.as_u64()}
    }
}

struct NetworkAddr {
    addr: String,
}

impl std::fmt::Display for NetworkAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.addr)
    }
}

impl From<&systemstat::IpAddr> for NetworkAddr {
    fn from(addrs: &systemstat::IpAddr) -> Self {
        NetworkAddr{ addr: match addrs {
            systemstat::IpAddr::V6(v6addr) => v6addr.to_string(),
            systemstat::IpAddr::V4(v4addr) => v4addr.to_string(),
            systemstat::IpAddr::Empty => "Empty".to_string(),
            systemstat::IpAddr::Unsupported => "Unsupported".to_string(),
        }}
    }
}

#[derive(Serialize, Deserialize)]
struct NetworkInfo {
    interfaces: HashMap<String, Vec<String>>,
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
struct InterfaceStatistics {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_packets: u64,
    tx_packets: u64,
    rx_errors: u64,
    tx_errors: u64,
}

#[cfg(unix)]
impl From<&systemstat::NetworkStats> for InterfaceStatistics {
    fn from(ns: &NetworkStats) -> Self {
        InterfaceStatistics {
            rx_bytes: ns.rx_bytes.as_u64(),
            tx_bytes: ns.tx_bytes.as_u64(),
            rx_packets: ns.rx_packets,
            tx_packets: ns.tx_packets,
            rx_errors: ns.rx_errors,
            tx_errors: ns.tx_errors,
        }
    }
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
struct NetworkStatistics {
    interfaces: HashMap<String, InterfaceStatistics>,
}

#[derive(Serialize, Deserialize)]
struct PowerInfo {
    has_battery: bool,
    battery_size: f32,
    remaining_time: u64,
    connect_to_ac: Option<bool>,
}

impl PowerInfo {
    fn new(battery_info: (bool, f32, u64), ac_info: Option<bool>) -> PowerInfo {
        PowerInfo{has_battery: battery_info.0, battery_size: battery_info.1, remaining_time: battery_info.2, connect_to_ac: ac_info}
    }
}

#[derive(Serialize, Deserialize)]
struct MemoryInfo {
    used: u64,
    total: u64,
}

impl From<&systemstat::Memory> for MemoryInfo {
    fn from(mem: &Memory) -> Self {
        MemoryInfo { used: systemstat::saturating_sub_bytes(mem.total, mem.free).as_u64(), total: mem.total.as_u64()}
    }
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
struct LoadAvg {
    last1: f32,
    last5: f32,
    last15: f32,
}

#[cfg(unix)]
impl From<&systemstat::LoadAverage> for LoadAvg {
    fn from(load: &LoadAverage) -> Self {
        LoadAvg{last1: load.one, last5: load.five, last15: load.fifteen}
    }
}

#[derive(Serialize, Deserialize)]
struct CpuLoadInfo {
    user: f32,
    system: f32,
    idle: f32,
}

impl From<&systemstat::CPULoad> for CpuLoadInfo {
    fn from(cpu: &CPULoad) -> Self {
        CpuLoadInfo {
            user: cpu.user * 100.0,
            system: cpu.system * 100.0,
            idle: cpu.idle * 100.0,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PostInfo {
    mount: Vec<MountInfo>,
    network: NetworkInfo,
    #[cfg(unix)]
    network_statistics: NetworkStatistics,
    power: PowerInfo,
    memory: MemoryInfo,
    cpu: CpuLoadInfo,
    #[cfg(unix)]
    loadavg: LoadAvg,
    uptime: u64,
    boot_time: String,
}


async fn measure_cpu(cpu: &systemstat::DelayedMeasurement<CPULoad>) -> anyhow::Result<CpuLoadInfo> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    let cpu = cpu.done().unwrap();
    Ok(CpuLoadInfo::from(&cpu))
}

pub fn get_base_info() {
    let sys = System::new();

    let mount = match sys.mounts() {
        Ok(mounts) => {
            let mut m: Vec<MountInfo> = Default::default();
            for mount in mounts.iter() {
                m.push(MountInfo::from(mount));
            }
            m
        }
        Err(x) => {
            error!("Got error in fetch mount: {}", x);
            Default::default()
        }
    };

    let network = match sys.networks() {
        Ok(netifs) => {
            let mut m: HashMap<String, Vec<String>> = Default::default();
            for netif in netifs.values() {
                let mut v: Vec<String> = Default::default();
                for addr in &netif.addrs {
                    v.push(NetworkAddr::from(&addr.addr).to_string());
                }
                m.insert(netif.name.clone(), v);
            }
            m
        }
        Err(x) => {
            error!("Got error in fetch network interface: {}", x);
            Default::default()
        }
    };

    #[cfg(unix)]
    let network_statistics = match sys.networks() {
        Ok(netifs) => {
            let mut map: HashMap<String, InterfaceStatistics> = Default::default();
            let mut if_stat_err: Vec<String> = Default::default();
            for netif in netifs.values() {
                let stats = sys.network_stats(&netif.name);
                if stats.is_ok() {
                    map.insert(netif.name.clone(), InterfaceStatistics::from(&stats.unwrap()));
                } else {
                    if_stat_err.push(netif.name.clone());
                }
            }
            map
        }
        Err(x) => {
            error!("Got error in fetch network statistics: {}", x);
            Default::default()
        }
    };

    let battery_info = match sys.battery_life() {
        Ok(battery) =>
            (true, battery.remaining_capacity, battery.remaining_time.as_secs()),
        Err(_x) => (false, 0f32, 0u64)
    };

    let power_info = PowerInfo::new(battery_info, match sys.on_ac_power() {
        Ok(power) => Some(power),
        Err(e) => {
            error!("Got error in fetch AC status: {}", e);
            None
        }
    });

    let memory_info = match sys.memory() {
        Ok(mem) => MemoryInfo::from(&mem),
        Err(x) => {
            println!("Got error in fetch memory usage: {}", x);
            MemoryInfo{total: 0, used: 0}
        }
    };

    #[cfg(unix)]
    let load_avg = match sys.load_average() {
        Ok(loadavg) => {
            LoadAvg::from(&loadavg)
        },
        Err(x) => {
            println!("Got error in load average: {}", x);
            LoadAvg{last1: 0.0, last5: 0.0, last15: 0.0}
        }
    };

    let uptime = sys.uptime().unwrap();

    let boot_time = sys.boot_time().unwrap().to_string();


}
