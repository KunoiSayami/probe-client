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
use crate::configparser::config::Configure;
use crate::configparser::config::*;
use crate::session::response::JsonResponse;
use anyhow::Result;
use log::info;
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use std::path::{Path};
use systemstat::Platform;
use std::time::Duration;
use std::fmt::Formatter;

pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_INTERVAL: u32 = 180;
pub const MAX_RETRY_TIMES: i32 = 3;

pub mod error {
    use std::fmt::Formatter;

    #[derive(Debug)]
    pub struct TooManyRetriesError {
        e: anyhow::Error
    }

    impl std::error::Error for TooManyRetriesError {

    }

    impl std::fmt::Display for TooManyRetriesError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Max retry times exceed! last error: {:?}", self.e)
        }
    }

    impl TooManyRetriesError {
        pub fn new(e: anyhow::Error) -> anyhow::Error {
            anyhow::Error::new(TooManyRetriesError {e})
        }
    }
}

mod response {
    use serde_derive::{Deserialize, Serialize};
    use std::fmt::Formatter;

    #[derive(Serialize, Deserialize)]
    pub struct JsonResponse {
        version: String,
        status: i64,
        #[deprecated(since= "1.5.0")]
        error_code: Option<i64>,
        message: Option<String>,
    }

    impl JsonResponse {
        pub fn get_status_code(&self) -> i64 {
            self.status
        }

        pub fn get_additional_message(&self) -> Option<String> {
            self.message.clone()
        }

        pub fn to_error(&self) -> Error {
            Error::from(self)
        }

        pub fn get_server_version(&self) -> &String {
            &self.version
        }
    }

    #[derive(Debug)]
    pub struct Error {
        code: i64,
        message: Option<String>,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Got response error: {} {}",
                self.code,
                self.message
                    .clone()
                    .unwrap_or_else(|| "(No description)".to_string())
            )
        }
    }

    impl std::error::Error for Error {}

    impl From<&JsonResponse> for Error {
        fn from(resp: &JsonResponse) -> Self {
            Error {
                code: resp.get_status_code(),
                message: resp.get_additional_message(),
            }
        }
    }
}

pub struct ServerAddress {
    address: Vec<String>,
    current_loc: usize
}

impl ServerAddress {
    fn new(cfg: &Configure) -> Self {
        let mut adr = vec![cfg.server.server_address.clone()];
        if let Some(servers) = cfg.server.backup_servers.clone() {
            adr.append(&mut servers.clone())
        }
        Self {
            address: adr,
            current_loc: usize::MAX
        }
    }

    fn get(&self) -> Option<&String> {
        if self.current_loc < self.len() && self.current_loc > 0{
            Some(&self.address[self.current_loc])
        } else {
            None
        }
    }

    fn get_unwrap(&self) -> &String {
        self.get().unwrap()
    }

    fn next(&mut self) -> Option<&String> {
        if self.current_loc == usize::MAX {
            self.current_loc = 0;
        } else {
            self.current_loc += 1;
        }
        self.get()
    }

    fn len(&self) -> usize {
        self.address.len()
    }
}

pub struct Session {
    config: Configure,
    client: reqwest::Client,
    server_version: String,
    server_address: ServerAddress,
}

#[derive(Debug)]
pub struct ExitProcessRequest {
    status_code: i64,
    message: String
}

impl std::error::Error for ExitProcessRequest {}

impl std::fmt::Display for ExitProcessRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Request exit process immediately (code: {}, message: {})", self.status_code, self.message)
    }
}

impl ExitProcessRequest {
    fn new<T: Into<String>>(status_code: i64, message: T) -> Self {
        Self {
            status_code,
            message: message.into()
        }
    }
}

impl From<&JsonResponse> for ExitProcessRequest {
    fn from(j: &JsonResponse) -> Self {
        Self::new(j.get_status_code(), j.get_additional_message().unwrap_or_else(|| "No additional message".to_string()))
    }
}

impl Session {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Session> {
        let contents = tokio::fs::read_to_string(&path).await?;
        let contents_str = contents.as_str();

        let mut config: Configure = toml::from_str(contents_str)?;

        let mut header_map = HeaderMap::new();

        if config.identification.is_none() {
            config.identification = Some(Identification {
                token: uuid::Uuid::new_v4().to_string(),
            });
            info!(
                "Generate new uuid identification token: {}",
                config.identification.clone().unwrap().token
            );
            tokio::fs::write(&path, toml::to_string(&config)?).await?;
        }

        header_map.append(
            "Authorization",
            format!("Bearer {}", &config.server.token).parse()?,
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(header_map)
            .redirect(reqwest::redirect::Policy::default())
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .build()?;
        let server_address = ServerAddress::new(&config);

        Ok(Session { config, client, server_version: "".to_string() , server_address})
    }

    pub async fn post(&self, data: &HashMap<String, String>) -> Result<reqwest::Response> {
        self
            .post_data_to_url(self.server_address.get_unwrap(), data)
            .await
    }

    pub fn call_next(&mut self) -> Option<&String> {
        self.server_address.next()
    }

    async fn post_data_to_url(
        &self,
        url: &str,
        data: &HashMap<String, String>,
    ) -> Result<reqwest::Response> {
        Ok(self.client.post(url).json(data).send().await?)
    }

    pub async fn send_data(&self, action: &str, body: Option<String>) -> Result<reqwest::Response> {
        let mut data: HashMap<String, String> = Default::default();
        for item in [
            ("version", CLIENT_VERSION),
            ("action", action),
            ("uuid", &self.config.identification.as_ref().unwrap().token),
        ]
        .iter()
        {
            data.insert((*item.0).to_string(), (*item.1).to_string());
        }
        if body.is_some() {
            data.insert("body".to_string(), body.unwrap());
        }
        self.post(&data).await
    }

    pub async fn init_connection(&mut self) -> Result<()> {
        let system = systemstat::System::new();

        let data = RegisterData {
            boot_time: system.boot_time().unwrap().timestamp(),
            hostname: gethostname::gethostname().to_str().unwrap().to_string(),
        };

        let resp = self
            .send_data("register", Some(serde_json::to_string(&data)?))
            .await?;
        let rep = self.check_response(resp).await?;
        if let Some(v) = self.config.server.check_server_version {
            if v {
                self.server_version = rep.get_server_version().clone();
            }
        }
        Ok(())
    }

    pub async fn send_heartbeat(&self) -> Result<()> {
        let resp = self
            .send_data(
                "heartbeat",
                if self.config.statistics.enabled {
                    Some(crate::info::get_base_info().await.to_string())
                } else {
                    None
                },
            )
            .await?;
        self.check_response(resp).await?;
        Ok(())
    }

    async fn check_response(&self, response: reqwest::Response) -> Result<JsonResponse> {
        let j: JsonResponse = response.json().await?;

        if !self.server_version.is_empty() && !self.server_version.eq(self.server_version.as_str()) {
            return Err(anyhow::Error::new(ExitProcessRequest::new(1, format!("Server version mismatch, except {} but {} found", &self.server_version, j.get_server_version()))))
        }
        match j.get_status_code() {
            200 => Ok(j),
            4031 | 4002 | 4000 => Err(anyhow::Error::new(ExitProcessRequest::from(&j))),
            _ => Err(anyhow::Error::new(j.to_error()))
        }
    }

    pub fn get_interval(&self) -> u64 {
        self.config.server.interval.clone().unwrap_or(DEFAULT_INTERVAL) as u64
    }
}
