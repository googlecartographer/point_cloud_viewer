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

use clap::Clap;
use octree_web_viewer::backend_error::PointsViewerError;
use octree_web_viewer::state::AppState;
use octree_web_viewer::utils::start_octree_server;
use point_viewer::data_provider::DataProviderFactory;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// HTTP web viewer for 3d points stored in OnDiskOctrees
#[derive(Clap, Debug)]
#[clap(name = "points_web_viewer", about = "Visualizing points")]
pub struct CommandLineArguments {
    /// The octree directory to serve, including a trailing slash.
    #[clap(name = "DIR", parse(from_os_str))]
    octree_path: PathBuf,
    /// Port to listen on.
    #[clap(default_value = "5433")]
    port: u16,
    /// IP string.
    #[clap(default_value = "127.0.0.1")]
    ip: String,
    #[clap(default_value = "100")]
    cache_items: usize,
}

/// init app state with command arguments
/// backward compatibilty is ensured
pub fn state_from(args: CommandLineArguments) -> Result<AppState, PointsViewerError> {
    // initial implementation: suffix from args not yet supported
    let suffix = PathBuf::new();
    let prefix = args.octree_path.parent().unwrap_or_else(|| Path::new(""));
    let data_provider_factory = DataProviderFactory::new();
    let octree_id = args.octree_path.strip_prefix(&prefix)?;
    Ok(AppState::new(
        args.cache_items,
        prefix,
        suffix,
        octree_id.to_str().unwrap(),
        data_provider_factory,
    ))
}

fn main() {
    let args = CommandLineArguments::parse();

    let ip_port = format!("{}:{}", args.ip, args.port);

    // initialize app state
    let app_state: Arc<AppState> = Arc::new(state_from(args).unwrap());
    // The actix-web framework handles requests asynchronously using actors. If we need multi-threaded
    // write access to the Octree, instead of using an RwLock we should use the actor system.
    // put octree arc in cache

    let sys = actix::System::new("octree-server");
    let _ = start_octree_server(app_state, &ip_port);

    eprintln!("Starting http server: {}", &ip_port);
    let _ = sys.run();
}
