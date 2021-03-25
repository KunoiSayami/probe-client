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
mod configparser;
mod info;
mod session;

use crate::session::Session;
use log::{error, info, warn};
use std::time::Duration;
use tokio::io::AsyncWriteExt as _;

async fn async_main(session: Session) -> anyhow::Result<()> {
    let interval = session.get_interval();
    session.init_connection().await?;
    loop {
        if let Err(e) = session.send_heartbeat().await {
            if e.is::<session::ExitProcessRequest>() {
                warn!("Got exit process request, break loop now");
                break
            }
            error!("Got error in send heartbeat: {:?}", e);
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
    }
    Ok(())
}

async fn retrieve_configure(sever_address: &str) -> anyhow::Result<()> {
    info!("retrieve configure from server");
    let client = reqwest::ClientBuilder::new()
        .user_agent(format!("probe_client_{}", session::CLIENT_VERSION))
        .build()?;

    let r = client.post(sever_address).send().await?;

    let response = r.text().await?;

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .open("data/probe_client.toml")
        .await?;

    file.write_all(response.as_bytes()).await?;
    file.sync_all().await?;
    info!("Write configure completed");
    Ok(())
}

async fn async_switch() -> anyhow::Result<()> {
    let args = clap::App::new("probe-client")
        .version(session::CLIENT_VERSION)
        .arg(
            clap::Arg::with_name("server_address")
                .short("r")
                .long("retrieve")
                .help("retrieve configure from specify remote server")
                .takes_value(true),
        )
        .get_matches();
    if let Some(server_addr) = args.value_of("server_address") {
        return retrieve_configure(server_addr).await
    }
    async_main(Session::new("data/probe_client.toml")?).await
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_switch())?;
    Ok(())
}
