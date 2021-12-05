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

use crate::session::{ReInitRequest, Session, MAX_RETRY_TIMES};
use log::{error, info, warn};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::AsyncWriteExt as _;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::session::error::TooManyRetriesError;

async fn post_main(session: &Session, rx: Arc<Mutex<mpsc::Receiver<()>>>) -> anyhow::Result<()> {
    let interval = session.get_interval();
    let mut rx = rx.lock().await;
    let mut times = 0;
    loop {
        if let Err(e) = session.send_heartbeat().await {
            if e.is::<session::ExitProcessRequest>() {
                warn!("Got exit process request, break loop now");
                break Err(e);
            }
            error!("Got error in send heartbeat: {:?}", e);
            if tokio::time::timeout(Duration::from_secs(5), rx.recv())
                .await
                .is_ok()
            {
                break Ok(());
            }
            if times > MAX_RETRY_TIMES {
                break Err(TooManyRetriesError::new(e));
            }
            times += 1;
            continue;
        }
        if tokio::time::timeout(Duration::from_secs(interval), rx.recv())
            .await
            .is_ok()
        {
            break Ok(());
        }
        times = 0;
    }
}

async fn retrieve_configure(sever_address: &str) -> anyhow::Result<()> {
    info!("retrieve configure from server");
    let client = reqwest::ClientBuilder::new()
        .user_agent(format!("probe_client {}", session::CLIENT_VERSION))
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

async fn async_main(mut session: Session, rx: mpsc::Receiver<()>) -> anyhow::Result<bool> {
    let arx = Arc::new(Mutex::new(rx));
    let mut return_value = false;
    while let Some(_) = session.call_next() {
        session.init_connection().await?;
        match post_main(&session, arx.clone()).await {
            Ok(()) => {
                return_value = true;
                break;
            }
            Err(e) if e.is::<TooManyRetriesError>() => {
                error!("{:?}", e);
                if session.check_is_last() {
                    return Err(e);
                }
                continue;
            }
            Err(e) if e.is::<ReInitRequest>() => {
                session.init_connection().await?;
                continue;
            }
            Err(e) => {
                error!("Got other error {:?}", e);
                return Err(e);
            }
        }
    }
    Ok(return_value)
}

async fn wait_ctrl_c(tx: mpsc::Sender<()>) -> anyhow::Result<()> {
    tokio::signal::ctrl_c().await?;
    tx.send(()).await.ok();
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
        .arg(
            clap::Arg::with_name("cfg")
                .short("c")
                .help("Specify configure file location")
                .takes_value(true),
        )
        .get_matches();
    if let Some(server_addr) = args.value_of("server_address") {
        return retrieve_configure(server_addr).await;
    }
    info!("Client version: {}", session::CLIENT_VERSION);
    let (tx, rx) = mpsc::channel(64);
    let session = Session::new(args.value_of("cfg").unwrap_or("data/probe_client.toml")).await?;
    let task = tokio::task::spawn(async_main(session, rx));
    let ctrl_c_task = tokio::task::spawn(wait_ctrl_c(tx));
    let result = task.await??;
    if !result {
        ctrl_c_task.abort();
    }
    match ctrl_c_task.await {
        Ok(r) => {
            if let Err(e) = r {
                error!("Got error while handle ctrl_c event: {:?}", e)
            }
        }
        Err(e) if !e.is_cancelled() => {
            error!("Got error while join task: {:?}", e);
        }
        _ => {}
    }
    Ok(())
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
