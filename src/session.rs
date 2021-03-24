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
use anyhow::Result;
use std::path::Path;
use reqwest::header::HeaderMap;
use std::collections::HashMap;
use log::{info};
use crate::configparser::config::*;
use systemstat::Platform;
use crate::session::response::JsonResponse;

pub const CLIENT_VERSION: &str = env!("CARGO_PKG_VERSION");



mod response {
    use serde_derive::{Deserialize, Serialize};
    use std::fmt::Formatter;

    #[derive(Serialize, Deserialize)]
    pub struct JsonResponse {
        status: i64,
        error_code: Option<i64>,
        message: Option<String>,
    }

    impl JsonResponse {
        pub fn get_status_code(&self) -> i64 {
            self.status
        }

        pub fn get_error_code(&self) -> Option<i64> {
            self.error_code
        }

        pub fn get_additional_message(&self) -> Option<String> {
            self.message.clone()
        }

        pub fn to_error(&self) -> Error {
            Error::from(self)
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
                code: resp
                    .get_error_code()
                    .unwrap_or_else(|| resp.get_status_code()),
                message: resp.get_additional_message(),
            }
        }
    }
}


pub struct Session {
    config: Configure,
    client: reqwest::Client,
}

impl Session {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Session> {
        let contents = std::fs::read_to_string(&path)?;
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
            std::fs::write(&path, toml::to_string(&config)?)?;
        }

        header_map.append(
            "Authorization",
            format!("Bearer {}", &config.server.token).parse()?,
        );

        let client = reqwest::ClientBuilder::new()
            .default_headers(header_map)
            .redirect(reqwest::redirect::Policy::default())
            .build()?;

        Ok(Session { config, client })
    }

    pub async fn post(&self, data: &HashMap<String, String>) -> Result<reqwest::Response> {
        match self
            .post_data_to_url(&self.config.server.server_address, data)
            .await
        {
            Ok(resp) => Ok(resp),
            Err(e) => {
                if let Some(servers) = &self.config.server.backup_servers {
                    let mut rt_value: Result<reqwest::Response> = Err(e);
                    for url in servers {
                        match self.post_data_to_url(url, data).await {
                            Ok(resp) => rt_value = Ok(resp),
                            Err(e) => rt_value = Err(e),
                        }
                    }
                    rt_value
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn post_data_to_url(
        &self,
        url: &str,
        data: &HashMap<String, String>,
    ) -> Result<reqwest::Response> {
        Ok(self.client.post(url).json(data).send().await?)
    }

    pub async fn send_data(
        &self,
        action: &str,
        body: Option<String>,
    ) -> Result<reqwest::Response> {
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

    pub async fn init_connection(&self) -> Result<()> {
        let system = systemstat::System::new();

        let data = RegisterData {
            boot_time: system.boot_time().unwrap().timestamp(),
            hostname: gethostname::gethostname().to_str().unwrap().to_string(),
        };

        let resp = self
            .send_data("register", Some(serde_json::to_string(&data)?))
            .await?;
        Session::check_response(resp).await
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
        Session::check_response(resp).await
    }

    async fn check_response(response: reqwest::Response) -> Result<()> {
        let j: JsonResponse = response.json().await?;
        if j.get_status_code() != 200 {
            Err(anyhow::Error::new(j.to_error()))
        } else {
            Ok(())
        }
    }

    pub fn get_interval(&self) -> u64 {
        self.config.server.interval.clone().unwrap_or(300) as u64
    }
}