/*-
 * #%L
 * OBKV Table Client Framework
 * %%
 * Copyright (C) 2021 OceanBase
 * %%
 * OBKV Table Client Framework is licensed under Mulan PSL v2.
 * You can use this software according to the terms and conditions of the
 * Mulan PSL v2. You may obtain a copy of Mulan PSL v2 at:
 *          http://license.coscl.org.cn/MulanPSL2
 * THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
 * KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
 * NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
 * See the Mulan PSL v2 for more details.
 * #L%
 */

extern crate bytes;
extern crate chrono;
extern crate crossbeam;
extern crate futures;
extern crate mysql;
#[macro_use]
extern crate serde;
extern crate serde_bytes;
extern crate tokio_codec;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate quick_error;
extern crate rand;
extern crate reqwest;
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate crypto;
extern crate futures_cpupool;
#[macro_use]
extern crate lazy_static;
extern crate prometheus;
extern crate r2d2;
extern crate scheduled_thread_pool;
extern crate spin;
extern crate uuid;
extern crate zstd;

#[macro_use]
mod macros;
pub mod client;
mod constant;
pub mod error;
mod location;
mod rpc;
pub mod serde_obkv;
mod util;
pub use self::{
    client::{
        query::{QueryResultSet, TableQuery},
        table::ObTable,
        table_client::{Builder, ObTableClient, RunningMode},
        ClientConfig, Table, TableOpResult,
    },
    proxy::OBKV_PROXY_HISTOGRAM_NUM_VEC,
    rpc::{
        protocol::{codes::ResultCodes, payloads, query},
        proxy, OBKV_RPC_HISTOGRAM_NUM_VEC, OBKV_RPC_HISTOGRAM_VEC,
    },
    serde_obkv::value::{ObjType, Value},
};
