// Copyright 2020 LEXUGE
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use super::{ClientWrapper, Result};
use async_trait::async_trait;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use trust_dns_client::{client::AsyncClient, udp::UdpClientStream};

/// Client instance for UDP connections
#[derive(Clone)]
pub struct Udp {
    addr: SocketAddr,
}

impl Udp {
    /// Create a new UDP client creator instance. with the given remote server address.
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }
}

#[async_trait]
impl ClientWrapper for Udp {
    async fn create(&self) -> Result<AsyncClient> {
        let stream = UdpClientStream::<UdpSocket>::new(self.addr);
        let (client, bg) = AsyncClient::connect(stream).await?;
        tokio::spawn(bg);
        Ok(client)
    }

    fn conn_type(&self) -> &'static str {
        "UDP"
    }
}
