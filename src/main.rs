// #![warn(rust_2018_idioms)]
#![allow(unused_mut)]
#![allow(unused_variables)]

use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, BytesCodec};

use futures::SinkExt;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use bytes::{BytesMut, /*BufMut, Bytes*/};
use std::time::{Instant};
use std::{thread, time};
// use std::str;
use tracing::{info, debug, error, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    use tracing_subscriber::{/*fmt::format::FmtSpan, EnvFilter,*/ FmtSubscriber};
    //tracing_subscriber::fmt()
    //   .with_env_filter(EnvFilter::from_default_env().add_directive("chat=info".parse()?))
    //    .with_span_events(FmtSpan::FULL)
    //    .init();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting subscriber failed");

    let state = Arc::new(Mutex::new(Shared::new()));
    // let game_state = Arc::new(Mutex::new(GameState {pos:HashMap::new(), names:HashMap::new()}));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:6142".to_string());

    // Note that this is the Tokio TcpListener, which is fully async.
    let listener = TcpListener::bind(&addr).await?;
    let udp_socket = UdpSocket::bind(&addr).await?;

    info!("server running on {}", addr);

    //let _ = tcp_accept_loop(listener, Arc::clone(&state), Arc::clone(&game_state)).await?;

    //let _ = udp_loop(udp_socket, Arc::clone(&game_state));
    
    
    let cloned_state = Arc::clone(&state);
    // let cloned_game_state = Arc::clone(&game_state);
    tokio::spawn(async move { let _ = tcp_accept_loop(listener, cloned_state); });

    // let cloned_game_state2 = Arc::clone(&game_state);
    tokio::spawn(async move { let _ = udp_loop(udp_socket); });

    let _ = game_loop(Arc::clone(&state)).await?;

    Ok(())
}

async fn tcp_accept_loop(listener: TcpListener, state: Arc<Mutex<Shared>>) -> Result<(), Box<dyn Error>> {
    println!("TCP Accept loop starting");
    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        // Clone a handle to the `Shared` state for the new connection.
        let state = Arc::clone(&state);
        // let game_state = Arc::clone(&game_state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            debug!("accepted connection");
            if let Err(e) = process(state, stream, addr).await {
                info!("an error occurred; error = {:?}", e);
            }
        });
    }
}

/// Shorthand for the transmit half of the message channel.
// type Tx = mpsc::UnboundedSender<String>;
type Tx = mpsc::UnboundedSender<BytesMut>;

/// Shorthand for the receive half of the message channel.
// type Rx = mpsc::UnboundedReceiver<String>;
type Rx = mpsc::UnboundedReceiver<BytesMut>;

// type Lobby = Vec<Tx>;

struct Shared {
    peers: HashMap<SocketAddr, Tx>,
}

/// The state for each connected client.
struct Peer {

    bytes: Framed<TcpStream, BytesCodec>,

    rx: Rx,
}

// struct GameState {
//     pos: HashMap<SocketAddr, (f32, f32)>,
//     names: HashMap<SocketAddr, String>,
// }

impl Shared {
    /// Create a new, empty, instance of `Shared`.
    fn new() -> Self {
        Shared {
            peers: HashMap::new(),
            // game_state: GameState{pos:HashMap::new(), names:HashMap::new()}
        }
    }

    async fn broadcast(&mut self, sender: SocketAddr, message: &BytesMut) {
        for peer in self.peers.iter_mut() {
            if *peer.0 != sender {
                let _ = peer.1.send(message.clone());
            }
        }
    }

    // async fn send(&mut self, sender: SocketAddr, pid: usize, message: &str) {
        
    // }
}

impl Peer {
    /// Create a new instance of `Peer`.
    async fn new(
        state: Arc<Mutex<Shared>>,
        // lines: Framed<TcpStream, LinesCodec>,
        bytes: Framed<TcpStream, BytesCodec>,
    ) -> io::Result<Peer> {
        // Get the client socket address
        // let addr = lines.get_ref().peer_addr()?;
        let addr = bytes.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (tx, rx) = mpsc::unbounded_channel();

        // Add an entry for this `Peer` in the shared state map.
        state.lock().await.peers.insert(addr, tx);

        // Ok(Peer { lines, rx })
        Ok(Peer { bytes, rx })
    }
}

/// Process an individual chat client
async fn process(
    state: Arc<Mutex<Shared>>,
    // game_state: Arc<Mutex<GameState>>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    // let lines = Framed::new(stream, LinesCodec::new());
    let bytes = Framed::new(stream, BytesCodec::new());

    let pid = COUNTER.fetch_add(1, Ordering::Relaxed);

    // TODO: send initial state


    // Register our peer with state which internally sets up some channels.
    // let mut peer = Peer::new(state.clone(), lines).await?;
    let mut peer = Peer::new(state.clone(), bytes).await?;

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer. Send it to the current user.
            Some(msg) = peer.rx.recv() => {
                peer.bytes.send(msg).await?;
            }
            result = peer.bytes.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(msg)) => {
                    let mut state = state.lock().await;
                    // let mut game_state = game_state.lock().await;
                    
                    let opcode = as_u16_le(&[msg[0], msg[1]]);
                    match opcode {
                        0 => println!("received 0"),
                        
                        _ => println!("did not receive 0"),
                    };

                    println!("{:?}", msg);

                    state.broadcast(addr, &msg).await;
                }
                // An error occurred.
                Some(Err(e)) => {
                    error!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        pid,
                        e
                    );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // If this section is reached it means that the client was disconnected!
    // Let's let everyone still connected know about it.
    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);

        //tracing::info!("{}", msg);
        //state.broadcast(addr, &msg).await;
    }

    Ok(())
}

// TODO pass in Shared ref so udp_loop can match users by their SocketAddr
async fn udp_loop(udp_socket: UdpSocket) -> Result<(), Box<dyn Error>> {
    println!("UDP Loop starting");
    let mut buf: [u8; 508] = [0; 508];

    loop {
        let (_size, socket) = udp_socket.recv_from(&mut buf).await?;

        // let mut state = state.lock().await;
        let opcode: u16 = as_u16_le(&buf[0.. 2]);
        match opcode {
            0 => println!("received 0"),
            _ => println!("did not receive 0"),
        };
    }
}

async fn game_loop(state: Arc<Mutex<Shared>>, /*udp_socket: UdpSocket*/) -> Result<(), Box<dyn Error>> {
    println!("Game Loop starting");
    let tick_rate = time::Duration::from_millis(100); //milliseconds
    
    loop {
        let start = Instant::now();
        
        // let game_state = game_state.lock().await;
        let mut state = state.lock().await;
        
        // TODO: broadcast game state
        // let pos: Vec<(f32, f32)> = game_state.pos.values().cloned().collect();
        // for peer in state.peers.iter_mut() {
        //     let _ = peer.1.send(42);
        //     //let _ = udp_socket.send_to(&pos, peer.0);
        // }

        let duration = start.elapsed();
        if duration < tick_rate {
            thread::sleep(tick_rate - duration);
        }
    }
}

fn as_u16_le(array: &[u8]) -> u16 {
    assert_eq!(array.len(), 2);
    ((array[0] as u16) <<  0) +
    ((array[1] as u16) <<  8)
}

// fn as_f32_le(array: &[u8]) -> f32 {
//     assert_eq!(array.len(), 4);
//     (((array[0] as u32) <<  0) +
//      ((array[1] as u32) <<  8) +
//      ((array[2] as u32) << 16) +
//      ((array[3] as u32) << 24)) as f32
// }

// fn bytes_to_string(array: &[u8]) -> String {
//     // TODO: handle error cases
//     let string_slice = str::from_utf8(array).unwrap();

//     String::from(string_slice)
// }

// fn vec_to_bytesmut(src: Vec<(f32, f32)>) -> BytesMut {
//     let mut bytes = BytesMut::new();

//     for i in src {
//         bytes.put_f32(i.0);
//         bytes.put_f32(i.1);
//     }

//     bytes
// }

//impl std::convert::From<Vec<(f32, f32)>> for u8 {
//    fn from(vec: Vec<(f32, f32)>) -> Self {
//        42;
//    }
//}
