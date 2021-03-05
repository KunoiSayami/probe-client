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
use std::time::Duration;
use anyhow::Error;
use log::error;

async fn async_main(session: Session) -> anyhow::Result<!> {
    session.init_connection().await?;
    loop {
        match session.send_heartbeat().await {
            Err(e) => {
                error!("Got error in send heartbeat: {:?}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue
            }
            _ => {}
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

fn main() -> anyhow::Result<!> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(configparser::config::Session::new(
            "data/config.toml",
        )?))?;
}
