// Collection of functions to interface with particld.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;

#[derive(Debug, Clone, Default)]
pub struct RPCURL(String);

impl RPCURL {
    pub fn target(
        mut self,
        ip: &str,
        port: u16,
        walletname: &str,
        user: &str,
        password: &str,
    ) -> Self {
        trace!("Constructing RPC console URL ...");
        if walletname.len() == 0 {
            if user != "" && password != "" {
                self.0 = format!("http://{}:{}@{}:{}/", user, password, ip, port);
            } else {
                self.0 = format!("http://{}:{}/", ip, port);
            }
        } else {
            if user != "" && password != "" {
                self.0 = format!(
                    "http://{}:{}@{}:{}/wallet/{}",
                    user, password, ip, port, walletname
                );
            } else {
                self.0 = format!("http://{}:{}/wallet/{}", ip, port, walletname);
            }
        }
        return self;
    }
}

fn parametrize(args: &str) -> Vec<Value> {
    trace!("Parsing arguments ...");
    let mut params: Vec<Value> = Vec::new();
    for entry in args.split(" ").collect::<Vec<&str>>() {
        match serde_json::from_str(entry) {
            Ok(val) => {
                params.push(val);
            }
            Err(_) => {
                params.push(Value::String(entry.to_string()));
            }
        }
    }
    return params;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RPCResponse {
    pub result: Value,
    pub error: Option<String>,
    pub id: String,
}

impl RPCResponse {
    fn unpack(self) -> Value {
        match self.error {
            Some(err) => {
                error!("{}", err);
                std::process::exit(1);
            }
            None => self.result,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Post<'r> {
    jsonrpc: &'r str,
    id: &'r str,
    method: Value,
    params: Value,
}

pub(crate) fn call(args: &str, rpcurl: &RPCURL) -> Result<Value, Box<dyn Error>> {
    let mut params = parametrize(args);
    let method = params[0].clone();
    params.remove(0);

    let post = Post {
        jsonrpc: "",
        id: "",
        method,
        params: Value::Array(params),
    };
    debug!("RPC: {} {} ...", &post.method, &post.params);
    let response: Value = ureq::post(&rpcurl.0)
        .set("Content-Type", "application/json")
        .send_json(serde_json::to_value(post)?)?
        .into_json()?;
    return Ok(response["result"].clone());
}
