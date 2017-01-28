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

use std::{io, slice, str, fmt};
use std::fs::{File, copy};
use std::io::{Error, Read};

use tokio_http2::http::{Request, Response, HttpProto};
use tokio_http2::StatusCode;
use rustc_serialize::json::*;
use rustc_serialize::base64::*;

// use multipart::server::{Multipart, Entries, SaveResult};
use tokio_http2::server::{Multipart, Entries, SaveResult};

pub fn route(req: Request, base_path: String) -> Response {
    match req.path() {
        "/admin/settings" => {
            post(req, base_path)
        },
        // NOTE: If the x-lambda-api header is set then route to the api::post
        _ => Response::new().with_status(StatusCode::MethodNotAllowed),
    }
}

fn post(req: Request, base_path: String) -> Response {
    let mut has_payload: bool = false;
    let mut content_lenth: u64 = 0;

    match req.content_type() {
        "application/json" => {
            match req.payload() {
                Some(payload) => {
                    let data = Json::from_str(str::from_utf8(payload).unwrap_or("{}"));
                    has_payload = true;
                    println!("{}", data.unwrap().pretty());
                },
                None => {},
            }
        },
        "application/base64" => {
            match req.payload() {
                Some(payload) => {
                    // Since the FromBase64 trait is in scope above you can apply it to payload()
                    let data = payload.from_base64();
                    has_payload = true;
                    println!("{:?}", data.unwrap());
                },
                None => {},
            }
        },
        "application/x-www-form-urlencoded" => {
            match req.payload() {
                Some(payload) => {
                    has_payload = true;
                    // println!("{:?}", req.urldecode(req.payload().unwrap_or("".as_bytes())));
                    //println!("{}", str::from_utf8(payload).unwrap_or(""));
                },
                None => {},
            }
        },
        "multipart/form-data" => {
            match Multipart::from_request(req) {
                Ok(mut multipart) => {
                    // Fetching all data and processing it.
                    // save_all() reads the request fully, parsing all fields and saving all files
                    // in a new temporary directory under the OS temporary directory.
                    match multipart.save_all() {
                        SaveResult::Full(entries) => {
                            has_payload = true;
                            process_entries(entries, base_path);
                        },
                        SaveResult::Partial(entries, error) => {
                            // Allow to fail below but contains some of the files.
                            process_entries(entries, base_path);
                        }
                        SaveResult::Error(error) => {},
                    }
                }
                Err(e) => {
                    println!("{:?}", e);
                }
            }
        },
        _ => {
            match req.payload() {
                Some(payload) => {
                    has_payload = true;
                    println!("{}", str::from_utf8(payload).unwrap_or(""));
                },
                None => {},
            }
        },
    }

    if has_payload {
        Response::new()
            .with_header("Server", "klaus")
            .with_header("Content-Length", &format!("{}", content_lenth))
            .with_status(StatusCode::Ok)
    } else {
        Response::new()
            .with_header("Server", "klaus")
            .with_header("Content-Length", &format!("{}", content_lenth))
            .with_status(StatusCode::BadRequest)
    }
}

fn process_entries(entries: Entries, base_path: String) -> u64 {
    // NOTE: What about fields??????
    for (name, field) in entries.fields {
        println!(r#"Field "{}": "{}""#, name, field);
    }

    let mut result_len: u64 = 0;

    for (name, savedfile) in entries.files {
        let filename = match savedfile.filename {
            Some(s) => s,
            None => "None".into()
        };

        match copy(savedfile.path, &format!("{}/{}", base_path, filename)) {
            Ok(len) => result_len += len,
            Err(e) => {},
        }
    }

    return result_len
}
