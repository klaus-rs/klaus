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

use tokio_http2::http::{Request, Response, HttpProto};
use tokio_http2::{StatusCode, Method};

use http::*;
use options::*;

pub fn routes(req: Request, base_path: String) -> Response {
    match req.method() {
        Method::Get | Method::Head => {
            // GET and HEAD are handled here...
            get_head::route(req, base_path)
        },
        Method::Post => {
            post::route(req, format!("{}/uploads", base_path)) // NOTE: Change this hardcode of upload after testing...
        },
        Method::Put => {
            //NB: Test with post method for now
            post::route(req, format!("{}/uploads", base_path)) // NOTE: Change this hardcode of upload after testing...
        },
        Method::Delete => {
            delete::route(req)
        },
        _ => {
            Response::new()
                .with_header("Server", "klaus")
                .with_header("Content-Length", "0")
                .with_status(StatusCode::NotImplemented)
        }
    }
}
