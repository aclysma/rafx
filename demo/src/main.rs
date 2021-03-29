// There's a decent amount of code that's just for example and isn't called
#![allow(dead_code)]

#[cfg(feature = "profile-with-tracy-memory")]
#[global_allocator]
static GLOBAL: profiling::tracy_client::ProfiledAllocator<std::alloc::System> =
    profiling::tracy_client::ProfiledAllocator::new(std::alloc::System, 100);

use structopt::StructOpt;

pub fn logging_init() {
    #[cfg(not(debug_assertions))]
    let log_level = log::LevelFilter::Info;
    #[cfg(debug_assertions)]
    let log_level = log::LevelFilter::Debug;

    // Setup logging
    env_logger::Builder::from_default_env()
        .default_format_timestamp_nanos(true)
        .filter_module(
            "rafx_assets::resources::descriptor_sets",
            log::LevelFilter::Info,
        )
        .filter_module("rafx_framework::nodes", log::LevelFilter::Info)
        .filter_module("rafx_framework::visibility", log::LevelFilter::Info)
        .filter_module("rafx_assets::graph", log::LevelFilter::Debug)
        .filter_module("rafx_framework::graph", log::LevelFilter::Debug)
        .filter_module("rafx_framework::resources", log::LevelFilter::Debug)
        .filter_module("rafx_framework::graph::graph_plan", log::LevelFilter::Info)
        .filter_module("rafx_api", log::LevelFilter::Debug)
        .filter_module("rafx_framework", log::LevelFilter::Debug)
        .filter_module("demo::phases", log::LevelFilter::Debug)
        .filter_module("mio", log::LevelFilter::Debug)
        // .filter_module(
        //     "rafx_assets::resources::command_buffers",
        //     log::LevelFilter::Trace,
        // )
        .filter_level(log_level)
        // .format(|buf, record| { //TODO: Get a frame count in here
        //     writeln!(buf,
        //              "{} [{}] - {}",
        //              chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        //              record.level(),
        //              record.args()
        //     )
        // })
        .init();
}

fn main() {
    logging_init();

    let args = demo::DemoArgs::from_args();

    demo::run(&args).unwrap();
}
