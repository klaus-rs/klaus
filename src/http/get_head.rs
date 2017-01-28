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

use tokio_http2::http::{Request, Response, HttpProto};
use tokio_http2::{StatusCode, Method};

use files::*;
use api::get_head;

pub fn route(req: Request, base_path: String) -> Response {
    let verb = req.method();

    match req.path() {
        "" => {
            Response::new()
                .with_header("Server", "klaus")
                .with_header("Content-Length", "0")
                .with_status(StatusCode::NoContent)
        },
        "/" | "/index.html" => {
            load_default(base_path, verb)
        },
        file_path => {
            match req.header("x-lambda-api") {
                Some(header) => {
                    let req = req.clone();
                    // api::get_head
                    get_head::route(req, &base_path, header)
                },
                None => {
                    match read(&format!("{}{}", base_path, file_path)) {
                        Ok(file_body) => {
                            if verb == Method::Get {
                                Response::new()
                                    .with_header("Server", "klaus")
                                    .with_header("Content-Length", &file_body.content_length.to_string())
                                    .with_header("Content-Type", &file_body.content_type)
                                    .with_body(file_body.body)
                                    .with_status(StatusCode::Ok)
                            } else {
                                Response::new()
                                    .with_header("Server", "klaus")
                                    .with_header("Content-Length", &file_body.content_length.to_string())
                                    .with_header("Content-Type", &file_body.content_type)
                                    .with_status(StatusCode::Ok)
                            }
                        },
                        Err(e) => {
                            // NOTE: This defaults to the index page so that single page apps like React can
                            // determine a 404 error or show page source.

                            if e.kind() == ErrorKind::NotFound {
                                load_default(base_path, verb)
                             } else {
                                Response::new()
                                    .with_header("Server", "klaus")
                                    .with_header("Content-Length", "0")
                                    .with_status(StatusCode::NotImplemented)
                            }
                        }
                    }
                }
            }
        },
    }
}

fn load_default(base_path: String, verb: Method) -> Response {
    // NOTE: Maybe add a list of acceptable index files in the config and check them in order instead of the static `index.html`
    match read(&format!("{}{}", base_path, "/index.html")) {
        Ok(file_body) => {
            if verb == Method::Get {
                Response::new()
                    .with_header("Server", "klaus")
                    .with_header("Content-Length", &file_body.content_length.to_string())
                    .with_header("Content-Type", &file_body.content_type)
                    .with_body(file_body.body)
                    .with_status(StatusCode::Ok)
            } else {
                Response::new()
                    .with_header("Server", "klaus")
                    .with_header("Content-Length", &file_body.content_length.to_string())
                    .with_header("Content-Type", &file_body.content_type)
                    .with_status(StatusCode::Ok)
            }
        },
        Err(e) => {
            Response::new()
                .with_header("Server", "klaus")
                .with_header("Content-Length", "0")
                .with_status(StatusCode::NotFound)
        }
    }
}
