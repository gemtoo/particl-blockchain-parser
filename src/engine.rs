use crate::{console::*, db, rpc::RPCURL};
use bitcoincore_zmq::{subscribe_async, Message, Message::HashBlock};
use clap::ArgMatches;
use futures_util::StreamExt;
use humantime::Duration;
use serde::{Deserialize, Serialize};
use std::error::Error;
use surrealdb::{engine::remote::ws::Client, Surreal};

pub async fn run(args: &ArgMatches) {
    let ipsplit: Vec<&str> = args
        .get_one::<String>("Particld IP")
        .unwrap()
        .split(":")
        .collect::<Vec<&str>>();
    if ipsplit.len() != 2 {
        error!("Particld IP parsing error.");
        std::process::exit(1);
    }
    let rpcurl = RPCURL::default().target(
        ipsplit[0],
        ipsplit[1].parse::<u16>().unwrap(),
        "",
        &args.get_one::<String>("user").unwrap(),
        &args.get_one::<String>("password").unwrap(),
    );
    let db = db::init(args).await;
    if let Err(e) = catchup(&db, &rpcurl).await {
        error!("{}", e);
        std::process::exit(1);
    }
    if let Err(e) = listen(&db, &rpcurl).await {
        error!("{}", e);
        std::process::exit(1);
    }
}

async fn scan(
    blockhash: &String,
    proposal_ids: &mut Vec<u64>,
    db: &Surreal<Client>,
    rpcurl: &RPCURL,
) -> Result<(), Box<dyn Error>> {
    let blockdata: BlockData = getblock(blockhash, db, &rpcurl).await?;
    if let Ok(Some(proposal)) = getnewproposal(&blockdata, &proposal_ids, &rpcurl).await {
        db::regproposal(&db, &proposal).await?;
        *proposal_ids = db::getproposalids(&db).await?;
    }
    db::regblock(&db, &blockdata).await?;
    Ok(())
}

async fn catchup(db: &Surreal<Client>, rpcurl: &RPCURL) -> Result<(), Box<dyn Error>> {
    info!("Catching up the blocks ...");
    let nextheight = match db::toprec(&db).await? {
        // Continue building database from last recorded block + 1.
        Some(thing) => thing + 1,
        // This is a start height from which database is going to be initialized.
        None => 0,
    };
    let mut proposal_ids = db::getproposalids(&db).await?;
    for height in nextheight.. {
        let blockhash_result = getblockhash(height, rpcurl).await;
        match blockhash_result {
            Ok(blockhash) => {
                scan(&blockhash, &mut proposal_ids, &db, &rpcurl).await?;
            }
            Err(e) => {
                error!("{}", e);
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessedBlocks {
    pub blocks: Vec<String>,
}

impl ProcessedBlocks {
    fn contains(&self, blockhash: &str) -> bool {
        self.blocks.iter().any(|e| blockhash == e)
    }
    fn inject(&mut self, blockhash: String) {
        self.blocks.push(blockhash);
        // ZMQ by default queues maximum 1000 transactions.
        if self.blocks.len() == 1001 {
            self.blocks.remove(0);
        }
    }
}

async fn listen(db: &Surreal<Client>, rpcurl: &RPCURL) -> Result<(), Box<dyn Error>> {
    let mut proposal_ids = db::getproposalids(&db).await?;
    let mut processed_blocks = ProcessedBlocks::default();
    if let Some(blocks) = db::gettrackedzmq(&db).await? {
        processed_blocks = blocks;
    }
    let mut stream = subscribe_async(&["tcp://particld:28332"])?;
    while let Some(msg) = stream.next().await {
        let blockhash = gethash(msg);
        if !processed_blocks.contains(&blockhash) {
            scan(&blockhash, &mut proposal_ids, &db, rpcurl).await?;
            processed_blocks.inject(blockhash);
            db::regtrackedzmq(&db, &processed_blocks).await?;
        }
    }
    Ok(())
}

fn gethash<E: Error + Sized>(msg: Result<Message, E>) -> String {
    match msg {
        Ok(msg) => match msg {
            HashBlock(hash, _) => {
                return hash.to_string();
            }
            _ => {
                error!("Got unexpected value from ZMQ.");
                std::process::exit(1);
            }
        },
        Err(e) => {
            error!("{}", e);
            std::process::exit(1);
        }
    }
}
