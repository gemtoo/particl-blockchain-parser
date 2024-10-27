use clap::{command, Arg, ArgMatches};

pub fn args() -> ArgMatches {
    let args = command!()
        .arg(
            Arg::new("Particld IP")
                .long("rpc-ip")
                .help("Particl RPC IP address to connect to, for example '127.0.0.1:51735'")
                .required(true),
        )
        .arg(
            Arg::new("user")
                .long("rpc-user")
                .help("Username for RPC authentication")
                .required(true),
        )
        .arg(
            Arg::new("password")
                .long("rpc-password")
                .help("Password for RPC authentication")
                .required(true),
        )
        .arg(
            Arg::new("stage")
                .long("stage")
                .help("Database name, for example 'prod'")
                .required(true),
        )
        .arg(
            Arg::new("SurrealDB IP")
                .long("surrealdb-ip")
                .help("IP address of the SurrealDB instance")
                .required(true),
        )
        .get_matches();

    return args;
}
