# Probe client

This project has stopped maintenance, please use [status-upstream](https://github.com/KunoiSayami/status-upstream.rs) instead.

# Configure

```toml
[server]

# Probe server address
server_address = "https://example.com:8888"

# Authorization token, used in 
token = ""

# Optional: backup servers
# backup_servers = [""]

# Optional: heartbeat interval
interval = 300

[statistics]
#Set report to server statistics in each report
enabled = false
```

## License

[![](https://www.gnu.org/graphics/agplv3-155x51.png)](https://www.gnu.org/licenses/agpl-3.0.txt)

Copyright (C) 2021-2022 KunoiSayami

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with this program. If not, see <https://www.gnu.org/licenses/>.
