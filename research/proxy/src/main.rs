use std::{
    collections::HashMap,
    fs,
    io::{self, Error, ErrorKind, Result},
    mem,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use ascalon_network::{ClientCodec, Framed, ServerCodec};
use ascalon_protocol::Encode;
use ascalon_protocol_schema::Protocol;
use bytes::{Buf as _, Bytes, BytesMut};
use futures_util::{SinkExt, Stream, StreamExt};
use openssl::{
    bn::{BigNum, BigNumContext, MsbOption},
    rand::rand_bytes,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};

mod message;

static CLIENT_DH_PARAMS: &[u8] = include_bytes!("../dh_params.bin");
static SERVER_DH_PARAMS: &[u8] = include_bytes!("../../dh_params.bin");

#[tokio::main]
async fn main() -> io::Result<()> {
    let _tracy = tracy_client::Client::start();

    let state = Arc::new(State {
        game_servers: RwLock::default(),
        protocols: ascalon_protocol_schema::parse(&fs::read_to_string("research/protocols.xml")?),
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

struct State {
    game_servers: RwLock<HashMap<u32, SocketAddr>>,
    protocols: Vec<Protocol>,
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

async fn proxy_auth(state: Arc<State>, client: TcpStream, packet: [u8; 16]) -> io::Result<()> {
    let mut server = TcpStream::connect("3.66.254.251:6112").await?;
    server.write_all(&packet).await?;
    proxy_connection(state, client, server, "Auth").await
}

async fn proxy_game(state: Arc<State>, mut client: TcpStream, packet: [u8; 16]) -> io::Result<()> {
    let mut packet = packet.to_vec();
    packet.resize(16 - 4 + 72, 0);
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

async fn proxy_connection(
    state: Arc<State>,
    mut client: TcpStream,
    mut server: TcpStream,
    protocol: &'static str,
) -> io::Result<()> {
    let client_key = server_dh_key_exchange(&mut client, &CLIENT_DH_PARAMS.try_into()?).await?;
    let client = Framed::new(client, ServerCodec::from_key(&client_key));
    let (mut client_sink, client_stream) = client.split();

    let server_key = client_dh_key_exchange(&mut server, &SERVER_DH_PARAMS.try_into()?).await?;
    let server = Framed::new(server, ClientCodec::from_key(&server_key));
    let (mut server_sink, server_stream) = server.split();

    let (to_client, mut client_rx) = mpsc::channel(32);
    let (to_server, mut server_rx) = mpsc::channel(32);

    tokio::select! {
        result = proxy_stream(
            state.clone(),
            client_stream,
            to_client.clone(),
            to_server.clone(),
            protocol,
            false,
        ) => result,

        result = proxy_stream(
            state,
            server_stream,
            to_server,
            to_client,
            protocol,
            true,
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

async fn proxy_stream<S>(
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
    let mut recv_buf = BytesMut::new();
    let mut send_buf = BytesMut::new();

    while let Some(data) = stream.next().await {
        recv_buf.extend_from_slice(&data?);

        loop {
            let mut tmp = &recv_buf[..];
            let mut message = match message::decode(&state.protocols, protocol, server, &mut tmp) {
                Ok(m) => m,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };

            recv_buf.advance(recv_buf.len() - tmp.len());

            if let Some(client) = tracy_client::Client::running() {
                client.color_message(
                    &format!("{message:?}"),
                    if server { 0x00FF00FF } else { 0xFF0000FF },
                    0,
                );
            }

            match (protocol, server, message.id) {
                ("Auth", true, 20) => {
                    state.game_servers.write().unwrap().insert(
                        *message["unknown2"].as_u32(),
                        mem::replace(
                            message["address"].as_address_mut(),
                            "127.0.0.1:0".parse().unwrap(),
                        ),
                    );
                }
                ("Game", true, 1066) => {
                    state.game_servers.write().unwrap().insert(
                        *message["unknown2"].as_u32(),
                        mem::replace(
                            message["address"].as_address_mut(),
                            "127.0.0.1:0".parse().unwrap(),
                        ),
                    );
                }
                _ => {}
            }

            message.encode(&mut send_buf)?;
        }

        dst.send(send_buf.split().freeze())
            .await
            .map_err(|_| io::ErrorKind::BrokenPipe)?;
    }

    Ok(())
}

fn bignum_from_le_bytes(bytes: &[u8]) -> BigNum {
    let mut bytes = bytes.to_vec();
    bytes.reverse();

    BigNum::from_slice(&bytes).unwrap()
}

struct ClientDhParams {
    g: BigNum,
    p: BigNum,
    y: BigNum,
}

impl TryFrom<&[u8]> for ClientDhParams {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        Ok(Self {
            g: bignum_from_le_bytes(value.get(0x04..0x08).ok_or(ErrorKind::UnexpectedEof)?),
            p: bignum_from_le_bytes(value.get(0x08..0x48).ok_or(ErrorKind::UnexpectedEof)?),
            y: bignum_from_le_bytes(value.get(0x48..0x88).ok_or(ErrorKind::UnexpectedEof)?),
        })
    }
}

struct ServerDhParams {
    p: BigNum,
    x: BigNum,
}

impl TryFrom<&[u8]> for ServerDhParams {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self> {
        Ok(Self {
            p: bignum_from_le_bytes(value.get(0x08..0x48).ok_or(ErrorKind::UnexpectedEof)?),
            x: bignum_from_le_bytes(value.get(0x88..0xC8).ok_or(ErrorKind::UnexpectedEof)?),
        })
    }
}

async fn client_dh_key_exchange(
    stream: &mut TcpStream,
    params: &ClientDhParams,
) -> Result<[u8; 20]> {
    let mut x = BigNum::new().map_err(Error::other)?;
    x.rand(512, MsbOption::MAYBE_ZERO, false)
        .map_err(Error::other)?;

    let mut ctx = BigNumContext::new().map_err(Error::other)?;

    let mut y = BigNum::new().map_err(Error::other)?;
    y.mod_exp(&params.g, &x, &params.p, &mut ctx)
        .map_err(Error::other)?;

    let mut s = BigNum::new().map_err(Error::other)?;
    s.mod_exp(&params.y, &x, &params.p, &mut ctx)
        .map_err(Error::other)?;

    let mut y = y.to_vec_padded(64).map_err(Error::other)?;
    y.reverse();

    let mut packet = [0u8; 66];
    packet[0] = 0;
    packet[1] = 66;
    packet[2..].copy_from_slice(&y);
    stream.write_all(&packet).await?;

    let mut packet = [0u8; 22];
    stream.read_exact(&mut packet).await?;

    let mut s = s.to_vec();
    s.reverse();

    let mut k = [0u8; 20];
    k.copy_from_slice(&packet[2..]);
    for (k, s) in k.iter_mut().zip(&s) {
        *k ^= *s;
    }

    Ok(k)
}

async fn server_dh_key_exchange(
    stream: &mut TcpStream,
    params: &ServerDhParams,
) -> Result<[u8; 20]> {
    let mut packet = [0u8; 66];
    stream.read_exact(&mut packet).await?;

    let mut y = packet[2..].to_vec();
    y.reverse();
    let y = BigNum::from_slice(&y).map_err(Error::other)?;

    let mut ctx = BigNumContext::new().map_err(Error::other)?;

    let mut s = BigNum::new().map_err(Error::other)?;
    s.mod_exp(&y, &params.x, &params.p, &mut ctx)
        .map_err(Error::other)?;

    let mut s = s.to_vec();
    s.reverse();

    let mut k = [0u8; 20];
    rand_bytes(&mut k).map_err(Error::other)?;

    let mut k_encrypted = k;
    for (k, s) in k_encrypted.iter_mut().zip(&s) {
        *k ^= *s;
    }

    let mut packet = [0u8; 22];
    packet[0] = 1;
    packet[1] = 22;
    packet[2..].copy_from_slice(&k_encrypted);
    stream.write_all(&packet).await?;

    Ok(k)
}
