// Copyright 2020, 2021 LEXUGE
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

mod parser;
#[cfg(test)]
mod tests;
mod worker;

use self::{parser::Parsed, worker::worker};
use anyhow::{Context, Result};
use droute::{
    builders::{RouterBuilder, UpstreamsBuilder},
    error::DrouteError,
    Router,
};
use log::*;
use ratelimit::Limiter;
use simple_logger::SimpleLogger;
use std::{net::SocketAddr, path::PathBuf, result::Result as StdResult, sync::Arc, time::Duration};
use structopt::StructOpt;
use tokio::{fs::File, io::AsyncReadExt, net::UdpSocket, signal};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "dcompass",
    about = "High-performance DNS server with freestyle routing scheme support and DoT/DoH functionalities built-in."
)]
struct DcompassOpts {
    /// Path to the configuration file. Use built-in if not provided.
    #[structopt(short, long, parse(from_os_str))]
    config: Option<PathBuf>,

    /// Set this flag to validate the configuration file only.
    #[structopt(short, long, parse(from_flag))]
    validate: bool,
}

async fn init(p: Parsed) -> StdResult<(Router, SocketAddr, LevelFilter, u32), DrouteError> {
    Ok((
        RouterBuilder::new(p.table, UpstreamsBuilder::new(p.upstreams, p.cache_size))
            .build()
            .await?,
        p.address,
        p.verbosity,
        p.ratelimit,
    ))
}

async fn serve(socket: Arc<UdpSocket>, router: Arc<Router>, mut ratelimit: Limiter) {
    loop {
        // Size recommended by DNS Flag Day 2020: "This is practical for the server operators that know their environment, and the defaults in the DNS software should reflect the minimum safe size which is 1232."
        let mut buf = [0; 1232];
        // On windows, some applications may go away after they got their first response, resulting in a broken pipe, we should discard errors on receiving/sending messages.
        let (_, src) = match socket.recv_from(&mut buf).await {
            Ok(r) => r,
            Err(e) => {
                warn!("Failed to receive query: {}", e);
                continue;
            }
        };

        let router = router.clone();
        let socket = socket.clone();
        tokio::spawn(async move {
            match worker(router, socket, &buf, src).await {
                Ok(_) => (),
                Err(e) => warn!("Handling query failed: {}", e),
            }
        });

        ratelimit.wait();
    }
}

// Multi-threading has memory issue. see also: https://github.com/bluejekyll/trust-dns/issues/777
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let args: DcompassOpts = DcompassOpts::from_args();

    // If the config path is manually specified with `-c` flag, we use it and any error should fail early.
    // If there is no specified config but there is `config.yaml` under the path where user is invoking `dcompass` (not the absolute path of the binary), then we shall try that config. If the file exists but we failed to read, this should fail. Otherwise, we shall use the default anyway.
    let config = if let Some(config_path) = args.config {
        let display_path = config_path.as_path().display();
        let mut file = File::open(config_path.clone())
            .await
            .with_context(|| format!("Failed to open the file specified: {}", display_path))?;
        let mut config = String::new();
        file.read_to_string(&mut config)
            .await
            .with_context(|| format!("Failed to read from the file specified: {}", display_path))?;
        println!("Using the config file specified: {}", display_path);
        config
    } else {
        let mut config_path = std::env::current_dir()?;
        config_path.push("config.yaml");
        let display_path = config_path.as_path().display();
        match File::open(config_path.clone()).await {
            // We have found the config and successfully opened it.
            Ok(mut file) => {
                let mut config = String::new();
                file.read_to_string(&mut config).await.with_context(|| {
                    format!("Failed to read from the file found: {}", display_path)
                })?;
                println!("Using the config under current path: {}", display_path);
                config
            }
            // No config found, using built-in.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("No config found or specified, using built-in config.");
                include_str!("../../configs/default.json").to_owned()
            }
            // Found but unable to open. We shall exit as this is intended.
            Err(e) => {
                return Err(e).with_context(|| {
                    format!("`config.yaml` found, but failed to open: {}", display_path)
                })
            }
        }
    };

    // Create whatever we need for get dcompass up and running.
    let (router, addr, verbosity, ratelimit) = init(
        serde_yaml::from_str(&config)
            .with_context(|| "Failed to parse the configuration file".to_string())?,
    )
    .await?;

    // If we are only required to validate the config, we shall be safe to exit now.
    if args.validate {
        println!("The configuration provided is valid.");
        return Ok(());
    }

    // Start logging
    SimpleLogger::new()
        .with_module_level("trust_dns_https::https_client_stream", LevelFilter::Off) // This module is quite chatty, we want to disable it.
        .with_level(verbosity)
        .init()?;

    let ratelimit = ratelimit::Builder::new()
        .capacity(1500) // TODO: to be determined if this is a proper value
        .quantum(ratelimit)
        .interval(Duration::new(1, 0)) // add quantum tokens every 1 second
        .build();

    info!("Dcompass ready!");

    let router = Arc::new(router);
    // Bind an UDP socket
    let socket = Arc::new(
        UdpSocket::bind(addr)
            .await
            .with_context(|| format!("Failed to bind to {}", addr))?,
    );

    Ok(tokio::select! {
    _ = serve(socket, router, ratelimit) => (),
    _ = signal::ctrl_c() => {log::info!("Ctrl-C received, shutting down");}
    })
}
