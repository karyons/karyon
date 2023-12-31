mod shared;

use std::sync::Arc;

use async_std::io;
use async_trait::async_trait;
use clap::Parser;
use smol::{channel, Executor};

use karyon_core::crypto::{KeyPair, KeyPairType};
use karyon_net::{Endpoint, Port};

use karyon_p2p::{
    protocol::{ArcProtocol, Protocol, ProtocolEvent, ProtocolID},
    ArcPeer, Backend, Config, P2pError, Version,
};

use shared::run_executor;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional list of bootstrap peers to start the seeding process.
    #[arg(short)]
    bootstrap_peers: Vec<Endpoint>,

    /// Optional list of peer endpoints for manual connections.
    #[arg(short)]
    peer_endpoints: Vec<Endpoint>,

    /// Optional endpoint for accepting incoming connections.
    #[arg(short)]
    listen_endpoint: Option<Endpoint>,

    /// Optional TCP/UDP port for the discovery service.
    #[arg(short)]
    discovery_port: Option<Port>,

    /// Username
    #[arg(long)]
    username: String,
}

pub struct ChatProtocol {
    username: String,
    peer: ArcPeer,
    executor: Arc<Executor<'static>>,
}

impl ChatProtocol {
    fn new(username: &str, peer: ArcPeer, executor: Arc<Executor<'static>>) -> ArcProtocol {
        Arc::new(Self {
            peer,
            username: username.to_string(),
            executor,
        })
    }
}

#[async_trait]
impl Protocol for ChatProtocol {
    async fn start(self: Arc<Self>) -> Result<(), P2pError> {
        let selfc = self.clone();
        let stdin = io::stdin();
        let task = self.executor.spawn(async move {
            loop {
                let mut input = String::new();
                stdin.read_line(&mut input).await.unwrap();
                let msg = format!("> {}: {}", selfc.username, input.trim());
                selfc.peer.broadcast(&Self::id(), &msg).await;
            }
        });

        let listener = self.peer.register_listener::<Self>().await;
        loop {
            let event = listener.recv().await.unwrap();

            match event {
                ProtocolEvent::Message(msg) => {
                    let msg = String::from_utf8(msg).unwrap();
                    println!("{msg}");
                }
                ProtocolEvent::Shutdown => {
                    break;
                }
            }
        }

        task.cancel().await;
        listener.cancel().await;
        Ok(())
    }

    fn version() -> Result<Version, P2pError> {
        "0.1.0, 0.1.0".parse()
    }

    fn id() -> ProtocolID {
        "CHAT".into()
    }
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    // Create a PeerID based on the username.
    let key_pair = KeyPair::generate(&KeyPairType::Ed25519);

    // Create the configuration for the backend.
    let config = Config {
        listen_endpoint: cli.listen_endpoint,
        peer_endpoints: cli.peer_endpoints,
        bootstrap_peers: cli.bootstrap_peers,
        discovery_port: cli.discovery_port.unwrap_or(0),
        enable_tls: true,
        ..Default::default()
    };

    // Create a new Executor
    let ex = Arc::new(Executor::new());

    // Create a new Backend
    let backend = Backend::new(&key_pair, config, ex.clone());

    let (ctrlc_s, ctrlc_r) = channel::unbounded();
    let handle = move || ctrlc_s.try_send(()).unwrap();
    ctrlc::set_handler(handle).unwrap();

    let ex_cloned = ex.clone();
    run_executor(
        async {
            let username = cli.username;

            // Attach the ChatProtocol
            let c = move |peer| ChatProtocol::new(&username, peer, ex_cloned.clone());
            backend.attach_protocol::<ChatProtocol>(c).await.unwrap();

            // Run the backend
            backend.run().await.unwrap();

            // Wait for ctrlc signal
            ctrlc_r.recv().await.unwrap();

            // Shutdown the backend
            backend.shutdown().await;
        },
        ex,
    );
}
