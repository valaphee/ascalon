use std::{
    collections::HashMap,
    fs,
    io::{self, ErrorKind},
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use ascalon_network::{
    ClientCodec, ClientParams, Framed, ServerCodec, ServerParams, client_dh_key_exchange,
    server_dh_key_exchange,
};
use ascalon_protocol::Encode;
use ascalon_protocol_schema::{Protocol, parse};
use bytes::{Buf, Bytes, BytesMut};
use futures_util::{SinkExt, Stream, StreamExt};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

use crate::dynamic::Message;

mod dynamic;

struct State {
    game_servers: RwLock<HashMap<u32, SocketAddr>>,
    protocols: Vec<Protocol>,
}

fn decode(
    buf: &mut BytesMut,
    protocols: &[Protocol],
    protocol: &str,
    server: bool,
) -> Option<Message> {
    let mut src = &buf[..];

    let message = dynamic::decode(protocols, protocol, server, &mut src).ok()?;

    buf.advance(buf.len() - src.len());

    Some(message)
}

fn encode(message: &Message) -> io::Result<Bytes> {
    let mut buf = BytesMut::new();

    message
        .encode(&mut buf)
        .map_err(|_| ErrorKind::InvalidData)?;

    Ok(buf.freeze())
}

async fn relay<S>(
    state: Arc<State>,
    mut stream: S,
    src: mpsc::Sender<Bytes>,
    dst: mpsc::Sender<Bytes>,
    protocol: &'static str,
    server: bool,
) -> io::Result<()>
where
    S: Stream<Item = io::Result<Bytes>> + Unpin,
{
    let mut buf = BytesMut::with_capacity(4096);

    while let Some(data) = stream.next().await {
        buf.extend_from_slice(&data?);

        while let Some(mut message) = decode(&mut buf, &state.protocols, protocol, server) {
            if let Some(client) = tracy_client::Client::running() {
                let (side, color) = match server {
                    false => ("Server", 0x00FF00FF),
                    true => ("Client", 0xFF0000FF),
                };

                client.color_message(&format!("{protocol}::{side}::{message:?}"), color, 0);
            }

            match (protocol, server, message.id) {
                ("Auth", false, 20) => {
                    state.game_servers.write().unwrap().insert(
                        *message["unknown2"].as_u32(),
                        *message["unknown4"].as_address(),
                    );
                    *message["unknown4"].as_address_mut() = "127.0.0.1:0".parse().unwrap();
                }
                ("Game", false, 1066) => {
                    state.game_servers.write().unwrap().insert(
                        *message["unknown2"].as_u32(),
                        *message["unknown0"].as_address(),
                    );
                    *message["unknown0"].as_address_mut() = "127.0.0.1:0".parse().unwrap();
                }
                _ => {}
            }

            dst.send(encode(&message)?)
                .await
                .map_err(|_| ErrorKind::BrokenPipe)?;
        }
    }

    Ok(())
}

async fn proxy_connection(
    state: Arc<State>,
    mut client: TcpStream,
    mut server: TcpStream,
    protocol: &'static str,
) -> io::Result<()> {
    let client_key =
        server_dh_key_exchange(&mut client, &ServerParams::from_bytes(CLIENT_DH)?).await?;
    let client = Framed::new(client, ServerCodec::from_key(&client_key));
    let (mut client_sink, client_stream) = client.split();

    let server_key =
        client_dh_key_exchange(&mut server, &ClientParams::from_bytes(SERVER_DH)?).await?;
    let server = Framed::new(server, ClientCodec::from_key(&server_key));
    let (mut server_sink, server_stream) = server.split();

    let (to_client, mut client_rx) = mpsc::channel(32);
    let (to_server, mut server_rx) = mpsc::channel(32);

    tokio::select! {
        result = relay(
            state.clone(),
            client_stream,
            to_client.clone(),
            to_server.clone(),
            protocol,
            true,
        ) => result,

        result = relay(
            state,
            server_stream,
            to_server,
            to_client,
            protocol,
            false,
        ) => result,

        result = async {
            while let Some(data) = client_rx.recv().await {
                client_sink.send(data).await?;
            }

            Ok(())
        } => result,

        result = async {
            while let Some(data) = server_rx.recv().await {
                server_sink.send(data).await?;
            }

            Ok(())
        } => result
    }
}

async fn proxy_auth(state: Arc<State>, client: TcpStream, packet: [u8; 16]) -> io::Result<()> {
    let mut server = TcpStream::connect("3.66.254.251:6112").await?;
    server.write_all(&packet).await?;
    proxy_connection(state, client, server, "Auth").await
}

async fn proxy_game(state: Arc<State>, mut client: TcpStream, packet: [u8; 16]) -> io::Result<()> {
    let extra_len = u32::from_le_bytes(packet[12..16].try_into().unwrap()) as usize;
    let extra_len = extra_len.checked_sub(4).ok_or(ErrorKind::InvalidData)?;
    if extra_len < 4 {
        return Err(ErrorKind::InvalidData.into());
    }

    let mut packet = packet.to_vec();
    packet.resize(16 + extra_len, 0);
    client.read_exact(&mut packet[16..]).await?;

    let unknown = u32::from_le_bytes(packet[0x10..0x14].try_into().unwrap());
    let mut address = state
        .game_servers
        .read()
        .unwrap()
        .get(&unknown)
        .copied()
        .ok_or(ErrorKind::NotFound)?;

    if address.port() == 0 {
        address.set_port(6112);
    }

    let mut server = TcpStream::connect(address).await?;
    server.write_all(&packet).await?;

    proxy_connection(state, client, server, "Game").await
}

async fn proxy(state: Arc<State>, mut client: TcpStream) -> io::Result<()> {
    let mut packet = [0; 16];
    client.read_exact(&mut packet).await?;

    match packet[1] {
        4 => proxy_auth(state, client, packet).await,
        5 => proxy_game(state, client, packet).await,
        _ => Err(ErrorKind::InvalidData.into()),
    }
}

static CLIENT_DH: &[u8] = include_bytes!("../dh.bin");
static SERVER_DH: &[u8] = include_bytes!("../../dh.bin");

#[tokio::main]
async fn main() -> io::Result<()> {
    let _tracy = tracy_client::Client::start();

    let state = Arc::new(State {
        game_servers: RwLock::default(),
        protocols: parse(&fs::read_to_string("research/protocols.xml")?),
    });

    let listener = TcpListener::bind("127.0.0.1:6112").await?;

    loop {
        let state = state.clone();
        let (client, _) = listener.accept().await?;

        tokio::spawn(async move {
            proxy(state, client).await.unwrap();
        });
    }
}
