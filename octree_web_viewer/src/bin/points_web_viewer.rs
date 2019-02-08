// Copyright 2016 Google Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate actix;
extern crate actix_web;
extern crate byteorder;
#[macro_use]
extern crate clap;

//extern crate env_logger;
extern crate octree_web_viewer;
extern crate point_viewer;

use actix_web::http::Method;
use actix_web::{
    server, //middleware,
    HttpRequest,
    HttpResponse,
};
use octree_web_viewer::backend::{NodesData, VisibleNodes};
use point_viewer::octree;
//use std::env;
use std::path::PathBuf;
use std::sync::Arc;

const INDEX_HTML: &'static str = include_str!("../../client/index.html");
const APP_BUNDLE: &'static str = include_str!("../../../target/app_bundle.js");
const APP_BUNDLE_MAP: &'static str = include_str!("../../../target/app_bundle.js.map");

const DEFAULT_PORT: &str = "5433";

fn index(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(INDEX_HTML)
}

fn app_bundle(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(APP_BUNDLE)
}

fn app_bundle_source_map(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(APP_BUNDLE_MAP)
}

// CAVEAT from actix docs
//Be careful with synchronization primitives like Mutex or RwLock.
//The actix-web framework handles requests asynchronously.
//By blocking thread execution, all concurrent request handling processes would block.
//If you need to share or update some state from multiple threads, consider using the actix actor system.

fn main() {
    // debug
    //::std::env::set_var("RUST_LOG", "actix_web=info");
    //::std::env::set_var("RUST_LOG", "actix_web=debug");
    //env::set_var("RUST_BACKTRACE", "1");
    //env_logger::init();

    let matches = clap::App::new("octree_web_viewer")
        .args(&[
            clap::Arg::with_name("port")
                .help("Port to listen on for connections.")
                .long("port")
                .default_value(DEFAULT_PORT)
                .takes_value(true),
            clap::Arg::with_name("octree_directory")
                .help("Input directory of the octree directory to serve.")
                .index(1)
                .required(true),
        ])
        .get_matches();

    let port = value_t!(matches, "port", u16).unwrap();
    let ip_port = format!("127.0.0.1:{}", port);
    let octree_directory = PathBuf::from(matches.value_of("octree_directory").unwrap());

    let my_octree: Arc<dyn octree::Octree> = {
        let my_octree = match octree::OnDiskOctree::new(octree_directory) {
            Ok(my_octree) => my_octree,
            Err(err) => panic!("Could not load octree: {}", err),
        };
        Arc::new(my_octree)
    };

    let sys = actix::System::new("octree-server");
    let my_octree = Arc::clone(&my_octree); //->shadowing to let the first outlive the closure
    let _ = server::new(move || {
        let octree_cloned_visible_nodes = Arc::clone(&my_octree);
        let octree_cloned_nodes_data = Arc::clone(&my_octree);
        actix_web::App::new()
            //.middleware(middleware::Logger::default()) //debug
            .resource("/", |r| r.method(Method::GET).f(index))
            .resource("/app_bundle.js", |r| r.method(Method::GET).f(app_bundle))
            .resource("/app_bundle.js.map", |r| {
                r.method(Method::GET).f(app_bundle_source_map)
            })
            .resource("/visible_nodes", |r| {
                r.method(Method::GET)
                    .h(VisibleNodes::new(octree_cloned_visible_nodes))
            })
            .resource("/nodes_data", |r| {
                r.method(Method::POST)
                    .h(NodesData::new(octree_cloned_nodes_data))
            })
    }) //todo error handling?
    .bind(&ip_port)
    .expect(&format!("Can not bind to {}", &ip_port))
    .start();

    println!("Starting http server: {}", &ip_port);
    let _ = sys.run();
}
