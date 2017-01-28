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

#![allow(dead_code)]

use tokio_http2::http::Request;
use options::Options;
use cache::*;

use redis::*;

pub fn rate_limited(req: &Request, cache_master: Option<String>) -> bool {
    let mut is_rate_limited = true;

    // No in memory cache so return false.
    if !cache_master.is_some() {
        return false;
    }

    let server: &str = &format!("redis://{}/", cache_master.unwrap());
    match cache_connect(server) {
        Ok(con) => {
            let _ : () = con.setbit("is_rate_limited", 0, false).unwrap();

            // Make the default true to rate_limit if the cache server has an issue
            is_rate_limited = con.getbit("is_rate_limited", 0).unwrap_or(true) as bool;
        },
        Err(e) => { /* put logging here */ },
    }

    is_rate_limited
}
