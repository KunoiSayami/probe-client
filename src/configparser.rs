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

pub(crate) mod config {

    use serde_derive::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct Configure {
        pub server: RemoteServer,
        pub statistics: Statistics,
        pub identification: Option<Identification>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct RemoteServer {
        pub server_address: String,
        pub token: String,
        pub backup_servers: Option<Vec<String>>,
        pub interval: Option<u32>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Statistics {
        pub enabled: bool,
    }

    #[derive(Clone, Serialize, Deserialize)]
    pub struct Identification {
        pub token: String,
    }


    #[derive(Serialize, Deserialize)]
    pub struct RegisterData {
        pub hostname: String,
        pub boot_time: i64,
    }
}
