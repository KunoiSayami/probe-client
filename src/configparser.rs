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

    use serde_derive::{Serialize, Deserialize};
    use std::collections::HashMap;

    use std::path::Path;
    use anyhow::Result;
    use reqwest::header::HeaderMap;
    use std::convert::TryInto;
    use reqwest::Request;
    use log::{error};

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
        client: reqwest::Client
    }

    impl Session {

        pub fn new<P: AsRef<Path>>(path: P) -> Result<Session> {
            let contents = std::fs::read_to_string(path)?;
            let contents_str = contents.as_str();

            let config: Configure = toml::from_str(contents_str)?;

            let mut header_map = HeaderMap::new();

            header_map.append("Authorization", format!("Bearer {}", &config.server.token).parse()?);

            let client = reqwest::ClientBuilder::new()
                .default_headers(header_map)
                .redirect(reqwest::redirect::Policy::max_value())
                .build()?;

            Ok(Session{config, client})

        }

        pub async fn post_data<T: TryInto<serde_json::Value>>(&self, data: &T) -> Result<reqwest::Response>  {
            match self.post_data_to_url(&self.config.server.server_address, data.try_into()?).await {
                Ok(resp) => Ok(resp),
                Err(e) => {
                    if let Some(servers) = &self.config.server.backup_servers {
                        for url in servers {
                            match self.post_data_to_url(&url, data).await {
                                Ok(resp) => resp,
                                Err(e) => continue
                            }
                        }
                    } else {
                        Err(e)
                    }
                }
            }
        }

        pub async fn post_data_to_url(&self, url: &String, data: &serde_json::Value) -> Result<reqwest::Response> {
            Ok(self.client.post(url)
                .json(data)
                .send()
                .await?
            )
        }

    }
}
