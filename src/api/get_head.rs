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

use std::io::{Error, ErrorKind};
use std::{io, slice, str, fmt};

use tokio_http2::http::{Request, Response, HttpProto};
use tokio_http2::StatusCode;

use files::*;

pub fn route(req: Request, prefix: &str, header: &str) -> Response {
    match req.path() {
        "/api/get/something" => {
            let req = req.clone();
            api(req, prefix, header)
        },
        _ => {
            Response::new()
                .with_header("Server", "klaus")
                .with_header("Content-Length", "0")
                .with_status(StatusCode::NotImplemented)
        }
    }
}

// NOTE: Create a function for each route
fn api(req: Request, prefix: &str, header: &str) -> Response {
    println!("{:?}", req.path());
    println!("{:?}", req.query());
    println!("{:?}", req.urldecode(req.payload().unwrap_or("".as_bytes())).unwrap());
    //req.urldecode(str::from_utf8(req.payload().unwrap_or("".as_bytes())).unwrap_or("")));

    Response::new()
        .with_header("Server", "klaus")
        .with_header("Content-Length", "0")
        .with_status(StatusCode::NotImplemented)
}
