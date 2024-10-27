#![allow(unused)]
#[macro_use]
extern crate log;
pub const CRATE_NAME: &str = module_path!();
mod args;
mod console;
mod db;
mod engine;
mod logger;
mod pools;
mod rpc;

#[tokio::main]
async fn main() {
    let args = args::args();
    logger::init();
    engine::run(&args).await;
}
