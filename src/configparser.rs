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
mod config {

    use serde_derive::{Deserialize, Serialize};
    use std::collections::HashMap;

    use anyhow::Result;
    use log::{error, info};
    use reqwest::header::HeaderMap;
    use reqwest::Request;
    use std::path::Path;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Configure {
        server: RemoteServer,
        statistics: Statistics,
        identification: Option<Identification>,
    }

    #[derive(Serialize, Deserialize)]
    struct RemoteServer {
        server_address: String,
        token: String,
        backup_servers: Option<Vec<String>>,
    }

    #[derive(Serialize, Deserialize)]
    struct Statistics {
        enabled: bool,
    }

    #[derive(Serialize, Deserialize)]
    struct Identification {
        token: String,
    }

    struct Session {
        config: Configure,
        client: reqwest::Client,
    }

    impl Session {
        pub fn new<P: AsRef<Path>>(path: P) -> Result<Session> {
            let contents = std::fs::read_to_string(path)?;
            let contents_str = contents.as_str();

            let mut config: Configure = toml::from_str(contents_str)?;

            let mut header_map = HeaderMap::new();

            if config.identification.is_none() {
                config.identification = Some(Identification {
                    token: uuid::Uuid::new_v4().to_string(),
                });
                info!(
                    "Generate new uuid identification token: {}",
                    config.identification.unwrap().token
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
            body: Option<&str>,
        ) -> Result<reqwest::Response> {
            let mut data: HashMap<String, String> = Default::default();
            for item in [
                ("action", action),
                ("token", &self.config.identification.as_ref().unwrap().token),
            ]
            .iter()
            {
                data.insert((*item.0).to_string(), (*item.1).to_string());
            }
            if body.is_some() {
                data.insert("body".to_string(), body.unwrap().into_string());
            }
            self.post_data(&data).await
        }

        pub async fn init_connection(&self) -> Result<reqwest::Response> {
            self.send_data("register", None).await
        }

        pub async fn send_heartbeat(&self) -> Result<reqwest::Response> {
            self.send_data("heartbeat", None).await
        }
    }
}
