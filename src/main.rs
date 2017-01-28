// Copyright 2016 LambdaStack All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(unused_must_use)]

// NOTE: This attribute only needs to be set once.
#![doc(html_logo_url = "https://lambdastackio.github.io/static/images/lambdastack-200x200.png",
       html_favicon_url = "https://lambdastackio.github.io/static/images/favicon.ico",
       html_root_url = "https://lambdastackio.github.io/klaus/klaus/index.html")]

//! HTTP/1.1:
//! This crate is a full multithreaded Async I/O HTTPd Server that support multipart uploads, Pipelining and more.
//! It can be used to implement your own HTTP server that allows for Rust Lambda like handlers.
//!
//! It can be used "out of the box" as a drop in replacement for a typical Web Server that handles
//! static content or Single Page Applications like Angular and ReactJS.
//!
//! There are many additional features being added including the following:
//!   1. TLS
//!   2. HTTP/2 with Multiplexing
//!   3. Web Socket protocol upgrading
//!   4. Well defined configuration system - [Done]
//!   5. More enhanced logging - [Done]
//!   6. Builtin AWS/Ceph S3 interfaces - [Plumbing done]
//!   7. gRPC/Protobuf
//!
//! Roadmap - The longer term goal is to create an application base that can also act as agent with
//! the ability to move to different machines and reconfigure itself for a given role as needed.
//!
//! Base Path - The default of this `public` which means to look for a sub-directory called public
//! under the directory where this application is located. Please change this passed in CLI value of -f
//! to something like `/var/www/html` or change it in the config file.
//!
//! Naming paths - Never end a path with '/' but you can begin one with it. For example, do not
//! use this: `/home/me/server/` but you can use something like this: `/home/me/server`. If '/' is
//! required at the end then the server will add it for you.
//!

extern crate futures;
extern crate num_cpus;
extern crate pretty_env_logger;
extern crate url;
//extern crate filetime;
extern crate toml;
extern crate rustc_serialize;
extern crate mime_guess;
extern crate daemonize;
#[macro_use] extern crate log;
#[macro_use] extern crate clap;
#[macro_use] extern crate mime;
#[macro_use] extern crate lsio;
// extern crate aws_sdk_rust;

extern crate tokio_http2;
extern crate tokio_proto;
extern crate tokio_service;
extern crate tokio_core;
extern crate tokio_tls;
extern crate native_tls;

extern crate redis;

use std::io;
use std::env;
use std::path::PathBuf;
use std::convert::AsRef;
use std::time::Duration;

use daemonize::{Daemonize};

use futures::future;
use tokio_proto::TcpServer;
use tokio_service::Service;
use tokio_core::net::TcpStream;
use tokio_tls::{TlsConnectorExt, TlsAcceptorExt};
use tokio_http2::http::{Request, Response, HttpProto};
use tokio_http2::{Router, Route, RouterBuilder, Method, StatusCode};
use tokio_http2::logger::{Logger, LoggerLevel};

use clap::Shell;
use url::Url;

use lsio::config::ConfigFile;

// use aws_sdk_rust::aws::errors::s3::S3Error;
// use aws_sdk_rust::aws::s3::endpoint::*;
// use aws_sdk_rust::aws::s3::s3client::S3Client;
// use aws_sdk_rust::aws::common::region::Region;
// use aws_sdk_rust::aws::common::credentials::{AwsCredentialsProvider, DefaultCredentialsProviderSync};
// use aws_sdk_rust::aws::common::request::DispatchSignedRequest;

use routes::routes;
use handlers::handlers;
use options::Options;
use rate_limiting::*;

mod cli;
mod config;
mod routes;
mod http;
mod files;
mod api;
mod handlers;
mod options;
mod cache;
mod rate_limiting;

// Used for outbound S3 calls (if used).
static DEFAULT_USER_AGENT: &'static str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

// Make generic so as to be easier to create new servers...
#[derive(Clone, Debug)]
struct HttpService {
    /// Base path is used to hold the actual base directory location on the server of the request.
    base_path: String,
    cache_master: Option<String>,  // Format of IP:<port> - port is optional and defaults to 6379
    rate_limiting: bool,
}

// Could put this into service_fn closure later...
impl Service for HttpService {
    type Request = Request;
    type Response = Response;
    type Error = io::Error;
    type Future = future::Ok<Response, io::Error>;

    fn call(&self, mut req: Request) -> Self::Future {

        // Impose rate limiting here if enabled
        if self.rate_limiting {
            if rate_limited(&req, self.cache_master.clone()) {
                return future::ok(
                    Response::new()
                        .with_header("Server", "klaus")
                        .with_header("Content-Length", "0")
                        .with_status(StatusCode::TooManyRequests)
                );
            }
        }

        match req.handler() {
            Some(handler) => future::ok(handler(req, self.base_path.clone())),
            None => future::ok(routes(req, self.base_path.clone())),
        }
    }
}

fn main() {
    pretty_env_logger::init(); // Used for lower level libaries that use env_logger.
    let app = env!("CARGO_PKG_NAME");
    let config_dir = "/etc/lsio";
    let version = format!("{}", crate_version!());

    let mut is_quiet: bool = false;
    let mut is_time: bool = false;
    let mut is_bench: bool = false;
    let mut is_bucket_virtual: bool = true;
    let mut is_daemonize: bool = true;
    let mut is_rate_limiting: bool = false;

    let matches = cli::build_cli(app, config_dir, &version).get_matches();

    if matches.is_present("generate-bash-completions") {
        cli::build_cli(app, config_dir, &version)
            .gen_completions_to(app, Shell::Bash, &mut io::stdout());
        ::std::process::exit(0);
    }

    // If the -q or --quiet flag was passed then shut off all output
    if matches.is_present("quiet") {
        is_quiet = true;
    }

    // If the -t or --time flag was passed then track operation time
    if matches.is_present("time") {
        is_time = true;
    }

    // If the -h or --bucket-virtual-host flag was passed then track operation time
    if matches.is_present("bucket-virtual-host") {
        is_bucket_virtual = false;
    }

    // If the -d or --daemonize flag was passed then run server in foreground
    if matches.is_present("daemonize") {
        is_daemonize = false;
    }

    // NOTE: Get parameters or config for region, signature etc
    // Safe to unwrap since a default value is passed in. If a panic occurs then the environment
    // does not support a home directory.

    // let region = match matches.value_of("region").unwrap().to_string().to_lowercase().as_ref() {
    //     "uswest1" => Region::UsWest1,
    //     "uswest2" => Region::UsWest2,
    //     "cnnorth1" => Region::CnNorth1,
    //     "eucentral1" => Region::EuCentral1,
    //     "euwest1" => Region::EuWest1,
    //     "saeast1" => Region::SaEast1,
    //     "apnortheast1" => Region::ApNortheast1,
    //     "apnortheast2" => Region::ApNortheast2,
    //     "apsouth1" => Region::ApSouth1,
    //     "apsoutheast1" => Region::ApSoutheast1,
    //     "apsoutheast2" => Region::ApSoutheast2,
    //     _ => Region::UsEast1,
    // };

    // Option so None will be return if nothing is passed in.
    let ep_str = matches.value_of("endpoint");
    let proxy_str = matches.value_of("proxy");
    let signature_str = matches.value_of("signature");
    let bench = matches.value_of("bench");
    let user_agent = matches.value_of("user-agent").unwrap_or(DEFAULT_USER_AGENT); // Used for outbound calls
    let ip = matches.value_of("ip").unwrap_or("127.0.0.1");
    let port = matches.value_of("port").unwrap_or("8000");
    let log_loc = matches.value_of("log").unwrap_or("/tmp");  // The default from the cli.rs should be present else this default
    let pid = matches.value_of("pid").unwrap_or("/tmp");  // The default from the cli.rs should be present else this default
    let run_as_user = matches.value_of("run-as-user").unwrap_or("nobody");
    let run_in_group = matches.value_of("run-in-group").unwrap_or("daemon");
    let base_path = matches.value_of("base-path").unwrap_or("public"); // Default 'public' means 'public' is relative to this app's location.
    let cache = matches.value_of("cache").unwrap_or("127.0.0.1:6379");
    let rate = matches.value_of("rate");

    if rate.is_some() {
        is_rate_limiting = true;
    }

    // Override some parameters when bench is specified...
    if bench.is_some() {
        is_quiet = true;
        is_time = true;
        is_bench = true;
    }

    // Set the config_file path to the default if a value is empty or set it to the passed in path value
    let config_dir = matches.value_of("config").unwrap();
    let mut config_file = PathBuf::new();

    // NOTE: May want to change the config name and location later...
    if config_dir.is_empty() {
        config_file.push("/etc/lsio/klaus.conf");
    } else {
        config_file.push(&format!("{}/{}", config_dir, "klaus.conf"));
    }

    // Config file location has a default of /etc/lsio/config but it can be changed
    let mut config = config::Config::from_file(config_file).unwrap_or(config::Config::default());

    // Let CLI args override any config setting if they exists.
    if ep_str.is_some() {
        config.set_endpoint(Some(Url::parse(ep_str.unwrap()).unwrap()));
    }

    if proxy_str.is_some() {
        config.set_proxy(Some(Url::parse(proxy_str.unwrap()).unwrap()));
    }

    // if ip.is_some() {
    //     config.set_ip(Some(ip.unwrap()));
    // }

    if signature_str.is_some() {
        config.set_signature(signature_str.unwrap().to_string());
    } else {
        config.set_signature("V4".to_string());
    }

    // let sign: String = config.signature.to_lowercase();
    // let provider = DefaultCredentialsProviderSync::new(None).unwrap();
    // let endpoint = Endpoint::new(region,
    //                              if sign == "v2" {Signature::V2} else {Signature::V4},
    //                              config.clone().endpoint,
    //                              config.clone().proxy,
    //                              Some(user_agent.to_string()),
    //                              Some(is_bucket_virtual));
    //
    // let mut s3client = S3Client::new(provider, endpoint);

    // Daemonize is used to aid in the process of creating a daemonized server for Linux based
    // systems. It also works for OSX. By default some of the options will require a high
    // security setting but you can specify config or command line options to override those defaults.
    // The primary defaults that may cause a permissions issue are:
    // 1. -d (specify this and daemonize will not even execute)
    // 2. -l (give a new log file location)
    // 3. If you do want daemonize then pay attention to PID file location, USER and GROUP
    if is_daemonize {
        // Pull some of these values from the config and/or cli
        let daemonize = Daemonize::new()
            .pid_file(&format!("{}/{}.pid", /*pid*/ ".", app)) // Every method except `new` and `start`
            .chown_pid_file(true)      // is optional, see `Daemonize` documentation
            .working_directory("/tmp") // for default behaviour.
            .user(run_as_user)
            .group(run_in_group) // Group name
            .group(2)        // or group id.
            .umask(0o777)    // Set umask, `0o027` by default.
            .privileged_action(|| "Executed before drop privileges");

        match daemonize.start() {
            Ok(_) => info!("Success, daemonized"),
            Err(e) => error!("{}", e),
        }
    }

    /// Logger is the container of the logging functions. This gets passed to HttpProto.
    /// Running the server using -d (--daemonize) option will run it in the foreground so logging
    /// output will show in the terminal window in addition to the normal log file.
    ///
    /// NB: Logging does impact TPS (Transactions Per Second). For the absolute fastest response,
    /// set the HttpProto::logger to None which will disable logging. When both logging to a file
    /// and logging to the terminal window, the TPS is impacted severely. However, file logging
    /// only impacts a small amount.
    let logger = Logger::new(Some(&format!("{}/{}.log", log_loc, app)));

    // TODO: Need ability to re-read config and set options.

    /// Router is the container of all of the routes which includes the Handler. The Handler
    /// the function that gets called for a specific route (req::path()) for a given Method.
    /// If the req::handler() is None then the server will use it's default route. If the hander
    /// returns a valid handler then it will be called instead of the server's default route.

    let router_builder: Option<RouterBuilder> = handlers("".to_string());

    /// HttpProto has state. You can pass in options to HttpProto and then pass it to HttpCodec and
    /// then on to decode. The tokio_http2::http::mod.rs file contains HttpProto and HttpCodec.
    /// To add your custom items to that simply copy those two structs and make mods or wrap them.
    ///
    /// If you don't want any custom route handlers then set the `router` value to None.

    let http_proto = HttpProto{ router: if router_builder.is_some() {Some(router_builder.unwrap().build())} else {None},
                                logger: Some(logger.clone()),
                              };

    let addr: &str = &format!("{}:{}", ip, port);
    let addr = addr.parse().unwrap();

    // Create the initial App loading message.
    logger.write(LoggerLevel::Info, format!("Application started and listening at: {}, version: {}", addr, version.clone()));

    let base = base_path.clone().to_string();
    let cache_master = cache.clone().to_string();

    /// TcpServer is the core of the server. It takes http_proto, number of cpus and HttpService as
    /// input and then begins it's magic!
    let mut srv = TcpServer::new(http_proto, addr);
    srv.threads(num_cpus::get()); // Creates listening threads based on the number of CPUs

    /// HttpService is passed a base_path which represents the base folder location for the root
    /// of the given site if static files are returned.
    srv.serve(move || {
        Ok(
            HttpService{
                base_path: format!("{}", base),
                rate_limiting: is_rate_limiting,
                cache_master: Some(format!("{}", cache_master)),
            }
        )
    }); // Could do closure here instead of the full Service above
}
