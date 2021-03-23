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
#![feature(never_type)]
mod configparser;
mod info;

use configparser::config::Session;
use log::error;
use std::time::Duration;

async fn async_main(session: Session) -> anyhow::Result<!> {
    let interval = session.get_interval();
    session.init_connection().await?;
    loop {
        if let Err(e) = session.send_heartbeat().await {
            error!("Got error in send heartbeat: {:?}", e);
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
}

fn main() -> anyhow::Result<!> {
    env_logger::init();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(configparser::config::Session::new(
            "data/probe_client.toml",
        )?))?;
}
