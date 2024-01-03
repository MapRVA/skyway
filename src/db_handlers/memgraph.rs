use rsmgclient::{ConnectParams, Connection, QueryParam};

pub fn create_connection(host, port) -> 
    let connect_params = ConnectParams {
        host: Some(cli.hostname),
        port: port,
        ..Default::default()
    };

    let mut connection = Connection::connect(&connect_params)
        .expect("connection parameters should describe an available database connection");
