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
use systemstat::{saturating_sub_bytes, LoadAverage, Platform, System};

#[derive(Serialize, Deserialize)]
struct MountInfo {
    mount_from: String,
    mount_type: String,
    mount_on: String,
    mount_avail: String,
    mount_total: String,
}

#[derive(Serialize, Deserialize)]
struct NetworkAddr {
    addr: String,
}

#[derive(Serialize, Deserialize)]
struct NetworkInfo {
    interfaces: HashMap<String, Vec<String>>,
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
struct ifNetworkStatic {
    rx_bytes: f32,
    tx_bytes: f32,
    rx_packets: i64,
    tx_packets: i64,
    rx_errors: i32,
    tx_errors: i32,
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
struct NetworkStatistics {
    interfaces: HashMap<String, ifNetworkStatic>,
}

#[derive(Serialize, Deserialize)]
struct PowerInfo {
    has_battery: bool,
    battery_size: i32,
    connect_to_ac: bool,
}

#[derive(Serialize, Deserialize)]
struct MemoryInfo {
    used: f32,
    total: f32,
}

#[cfg(unix)]
#[derive(Serialize, Deserialize)]
struct LoadAverage {
    last1: f32,
    last5: f32,
    last15: f32,
}

#[derive(Serialize, Deserialize)]
struct CpuLoadInfo {
    user: f32,
    system: f32,
    idle: f32,
}

#[derive(Serialize, Deserialize)]
struct PostInfo {
    mount: MountInfo,
    network: NetworkInfo,
    #[cfg(unix)]
    network_statistics: NetworkStatistics,
    power: PowerInfo,
    memory: MemoryInfo,
    cpu: CpuLoadInfo,
    #[cfg(unix)]
    loadavg: LoadAverage,
}

pub fn get_base_info() {}
