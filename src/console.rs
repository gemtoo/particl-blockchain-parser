use crate::{
    console::Vout::Data,
    db,
    pools::{Pool, POOLS},
    rpc::{call, RPCURL},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, error::Error};
use surrealdb::{engine::remote::ws::Client, Surreal};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub bits: String,
    pub blocksig: Option<String>,
    pub chainwork: String,
    pub difficulty: f64,
    pub hash: String,
    pub hashproofofstake: Option<String>,
    pub height: u64,
    pub mediantime: u64,
    pub merkleroot: String,
    #[serde(rename(deserialize = "nTx", serialize = "nTx"))]
    pub n_tx: u64,
    pub nonce: u64,
    pub previousblockhash: Option<String>,
    pub prevstakemodifier: Option<String>,
    pub size: u64,
    pub stakekernelblockhash: Option<String>,
    pub stakekernelscript: Option<String>,
    pub stakekernelvalue: Option<f64>,
    pub strippedsize: u64,
    pub time: u64,
    pub tx: Vec<Transaction>,
    pub version: u64,
    #[serde(rename(deserialize = "versionHex", serialize = "versionHex"))]
    pub version_hex: String,
    pub weight: u64,
    pub witnessmerkleroot: String,
    pub coldstaking: Option<Pool>,
    pub voting_info: Option<Vote>,
}

impl BlockData {
    async fn determine_coldstaking(
        &mut self,
        db: &Surreal<Client>,
        rpcurl: &RPCURL,
    ) -> Result<(), Box<dyn Error>> {
        let hasstakeaddress: Option<Vec<String>> = match self.tx[0].vout[1].clone() {
            Vout::Standard {
                n: _,
                vout_type: _,
                value: _,
                valuesat: _,
                scriptpubkey,
            } => scriptpubkey.stakeaddresses,
            _ => {
                error!("Unexpected type of vout when validating address.");
                std::process::exit(1);
            }
        };
        match hasstakeaddress {
            Some(unchecked_raw_stakeaddresses) => {
                let coldstaking =
                    check_stakeaddress_in_db(&unchecked_raw_stakeaddresses[0], db, rpcurl).await?;
                self.coldstaking = Some(coldstaking);
                Ok(())
            }
            None => {
                self.coldstaking = None;
                Ok(())
            }
        }
    }
    fn read_vote(&mut self) {
        let vout = self.tx[0].vout[0].clone();
        match vout {
            Data {
                n: _,
                data_hex: _,
                smsgdifficulty: _,
                smsgfeerate: _,
                treasury_fund_cfwd: _,
                vout_type: _,
                vote,
            } => match vote {
                Some(content) => {
                    let parsed: Vec<u64> = content
                        .split(", ")
                        .map(|x| x.parse::<u64>().unwrap())
                        .collect();
                    if parsed.len() != 2 {
                        error!("Sanity checks for parsed vote stats failed.");
                        std::process::exit(1);
                    }
                    self.voting_info = Some(Vote {
                        proposal_id: parsed[0],
                        voted_for_option: parsed[1],
                    });
                }
                None => {
                    self.voting_info = None;
                }
            },
            _ => {
                self.voting_info = None;
            }
        }
    }
}

async fn check_stakeaddress_in_db(
    unchecked_raw_stakeaddress: &String,
    db: &Surreal<Client>,
    rpcurl: &RPCURL,
) -> Result<Pool, Box<dyn Error>> {
    let known_stakeaddresses = db::getstakeaddresses(db).await?;
    trace!("Checking for known stakeaddresses ...");
    let mut coldstaking = Pool::default();
    // Loop through known addresses and return if there is one
    for known_stakeaddress in known_stakeaddresses.iter() {
        if &known_stakeaddress.raw == unchecked_raw_stakeaddress {
            trace!("Known stakeaddress found. Skipping address validation.");
            coldstaking = known_stakeaddress.pool.clone();
            return Ok(coldstaking);
        }
    }
    trace!("No known stakeaddresses found.");
    let coldstaking = validateaddress(unchecked_raw_stakeaddress, db, rpcurl).await?;
    Ok(coldstaking)
}
async fn validateaddress(
    stakeaddress: &str,
    db: &Surreal<Client>,
    rpcurl: &RPCURL,
) -> Result<Pool, Box<dyn Error>> {
    info!("Validating address ...");
    let arg = format!("validateaddress {} true", stakeaddress);
    let value = call(&arg, rpcurl)?;
    let poolkey: String = serde_json::from_value(value["stakeonly_address"].clone()).unwrap();
    // Default is no pool.
    let mut coldstaking = Pool {
        pubkey: poolkey.clone(),
        url: None,
        pool_is_active: None,
    };
    for known_pool in POOLS {
        if &poolkey == known_pool.pubkey {
            trace!("Stakeaddress belongs to a known pool.");
            coldstaking = known_pool.getpool();
            let stakeaddr_for_db = Stakeaddress {
                raw: stakeaddress.to_string(),
                pool: coldstaking.clone(),
            };
            db::regstakeaddress(db, &stakeaddr_for_db).await?;
            return Ok(coldstaking);
        }
    }
    trace!("Stakeaddress is of an unknown origin.");
    let stakeaddr_for_db = Stakeaddress {
        raw: stakeaddress.to_string(),
        pool: coldstaking.clone(),
    };
    db::regstakeaddress(db, &stakeaddr_for_db).await?;
    Ok(coldstaking)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stakeaddress {
    pub raw: String,
    pub pool: Pool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub hash: String,
    pub version: u64,
    pub size: u64,
    pub vsize: u64,
    pub weight: u64,
    pub locktime: u64,
    pub hex: String,
    pub vin: Vec<Vin>,
    pub vout: Vec<Vout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Vout {
    Data {
        n: u64,
        data_hex: String,
        smsgdifficulty: Option<String>,
        smsgfeerate: Option<f64>,
        treasury_fund_cfwd: Option<f64>,
        #[serde(rename(deserialize = "type", serialize = "type"))]
        vout_type: String,
        vote: Option<String>,
    },
    Standard {
        n: u64,
        #[serde(rename(deserialize = "type", serialize = "type"))]
        vout_type: String,
        value: f64,
        #[serde(rename(deserialize = "valueSat", serialize = "valueSat"))]
        valuesat: u64,
        #[serde(rename(deserialize = "scriptPubKey", serialize = "scriptPubKey"))]
        scriptpubkey: ScriptPubKey,
    },
    Blind {
        n: u64,
        #[serde(rename(deserialize = "type", serialize = "type"))]
        vout_type: String,
        pubkey: Option<String>,
        #[serde(rename(deserialize = "valueCommitment", serialize = "valueCommitment"))]
        value_commitment: String,
        data_hex: String,
        rangeproof: String,
    },
    Anon {
        n: u64,
        #[serde(rename(deserialize = "type", serialize = "type"))]
        vout_type: String,
        pubkey: Option<String>,
        #[serde(rename(deserialize = "valueCommitment", serialize = "valueCommitment"))]
        value_commitment: String,
        data_hex: String,
        rangeproof: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPubKey {
    pub addresses: Option<Vec<String>>,
    pub stakeaddresses: Option<Vec<String>>,
    pub asm: String,
    pub hex: String,
    #[serde(rename(deserialize = "reqSigs", serialize = "reqSigs"))]
    pub req_sigs: Option<u64>,
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub staking_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Vin {
    Standard {
        txid: String,
        vout: u64,
        #[serde(rename(deserialize = "scriptSig", serialize = "scriptSig"))]
        script_sig: ScriptSig,
    },
    Anon {
        #[serde(rename(deserialize = "type", serialize = "type"))]
        input_type: String,
        num_inputs: u64,
        ring_size: u64,
        txinwitness: Vec<String>,
        sequence: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptSig {
    pub asm: String,
    pub hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConclusion {
    pub isvalid: bool,
    pub stakeonly_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub proposal_id: u64,
    pub voted_for_option: u64,
}

impl Vote {
    pub async fn gen_proposal(&self, rpcurl: &RPCURL) -> Result<Proposal, Box<dyn Error>> {
        Ok(Proposal {
            proposal_id: self.proposal_id,
            stats: self.count_stats(rpcurl).await?,
        })
    }
    async fn count_stats(
        &self,
        rpcurl: &RPCURL,
    ) -> Result<HashMap<String, (u64, f64)>, Box<dyn Error>> {
        Ok(tallyvotes(*&self.proposal_id, rpcurl).await?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub proposal_id: u64,
    pub stats: HashMap<String, (u64, f64)>,
}

async fn tallyvotes(
    proposal_id: u64,
    rpcurl: &RPCURL,
) -> Result<HashMap<String, (u64, f64)>, Box<dyn Error>> {
    // 616959 is the block at which the first vote was recorded. Hence the minimum for the range in tallyvotes.
    let arg = format!("tallyvotes {} 616958 {}", proposal_id, i32::MAX);
    let context = call(&arg, rpcurl)?;
    let rawmap: HashMap<String, Value> = serde_json::from_value(context)?;
    let mut hmap: HashMap<String, (u64, f64)> = rawmap
        .iter()
        .map(|val| {
            (
                val.0.to_string().replace('"', ""),
                parse_tallyvotes_ratios(val.1.to_string().replace('"', "")),
            )
        })
        .collect();
    hmap.remove_entry("proposal").unwrap();
    hmap.remove_entry("blocks_counted").unwrap();
    hmap.remove_entry("height_start").unwrap();
    hmap.remove_entry("height_end").unwrap();
    Ok(hmap)
}

fn parse_tallyvotes_ratios(raw: String) -> (u64, f64) {
    let vote_stats_iterator = raw.split(", ");
    let mut index = 0;
    let mut vote_args_tuple: (u64, f64) = (0, 0.0);
    for vote_stat in vote_stats_iterator {
        if index == 0 {
            vote_args_tuple.0 = vote_stat.replace("%", "").trim().parse::<u64>().unwrap();
        } else if index == 1 {
            vote_args_tuple.1 = vote_stat.replace("%", "").trim().parse::<f64>().unwrap();
        }
        index += 1;
    }
    return vote_args_tuple;
}

pub async fn getblockhash(height: u64, rpcurl: &RPCURL) -> Result<String, Box<dyn Error>> {
    let arg = format!("getblockhash {}", height);
    let raw = call(&arg, rpcurl)?;
    let hash: String = serde_json::from_value(raw)?;
    Ok(hash)
}

pub async fn getblock(
    blockhash: impl Into<String>,
    db: &Surreal<Client>,
    rpcurl: &RPCURL,
) -> Result<BlockData, Box<dyn Error>> {
    let arg = format!("getblock {} 2 true", blockhash.into());
    let value = call(&arg, rpcurl)?;
    let mut blockdata: BlockData = serde_json::from_value(value)?;
    blockdata.determine_coldstaking(db, rpcurl).await?;
    blockdata.read_vote();
    Ok(blockdata)
}

pub async fn getnewproposal(
    blockdata: &BlockData,
    proposal_ids: &Vec<u64>,
    rpcurl: &RPCURL,
) -> Result<Option<Proposal>, Box<dyn Error>> {
    // 616959 is the block at which the first vote was recorded. Vote must have an associated proposal.
    if blockdata.height >= 616959 {
        match blockdata.voting_info.clone() {
            Some(vote) => {
                let existsyet = proposal_ids.iter().any(|&x| x == vote.proposal_id);
                if !existsyet {
                    let proposal = vote.gen_proposal(rpcurl).await?;
                    Ok(Some(proposal))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    } else {
        Ok(None)
    }
}
