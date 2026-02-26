//! Optional CLI binary for ClawRTC (feature-gated behind `cli`).
//!
//! Provides `clawrtc install`, `clawrtc start`, `clawrtc wallet create`, etc.

#[cfg(feature = "cli")]
fn main() {
    use clap::{Parser, Subcommand};
    use colored::Colorize;

    #[derive(Parser)]
    #[command(name = "clawrtc", version, about = "RustChain (RTC) miner and wallet CLI")]
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }

    #[derive(Subcommand)]
    enum Commands {
        /// Install the miner to ~/.clawrtc/
        Install {
            /// Wallet name
            #[arg(long, default_value = "default")]
            wallet: String,
            /// Skip prompts
            #[arg(long)]
            yes: bool,
        },
        /// Start the miner
        Start,
        /// Stop the miner
        Stop,
        /// Show miner status
        Status,
        /// Wallet management
        Wallet {
            #[command(subcommand)]
            action: WalletAction,
        },
    }

    #[derive(Subcommand)]
    enum WalletAction {
        /// Create a new wallet
        Create {
            #[arg(long)]
            force: bool,
        },
        /// Show wallet address and balance
        Show,
        /// Export wallet (public key only by default)
        Export {
            #[arg(long)]
            output: Option<String>,
        },
    }

    let cli = Cli::parse();

    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    match cli.command {
        Commands::Install { wallet, yes } => {
            println!("{}", "ClawRTC Installer".green().bold());
            println!("Wallet: {wallet}");
            if !yes {
                println!("Use --yes to skip prompts");
            }
            // Create wallet if needed
            let path = dirs::home_dir()
                .unwrap_or_default()
                .join(".clawrtc/wallets")
                .join(format!("{wallet}.json"));
            if !path.exists() {
                let w = openfang_clawrtc::RtcWallet::generate();
                w.save_plaintext(&path).expect("Failed to save wallet");
                println!("{} {}", "Wallet created:".green(), w.address());
            } else {
                let w = openfang_clawrtc::RtcWallet::from_file(&path).expect("Failed to load wallet");
                println!("{} {}", "Wallet exists:".yellow(), w.address());
            }
            println!("{}", "Installation complete.".green());
        }
        Commands::Start => {
            println!("{}", "Starting miner...".green());
            let path = dirs::home_dir()
                .unwrap_or_default()
                .join(".clawrtc/wallets/default.json");
            let wallet = openfang_clawrtc::RtcWallet::from_file(&path)
                .expect("No wallet found. Run: clawrtc install");

            let config = openfang_clawrtc::miner::MinerConfig {
                node_url: openfang_clawrtc::DEFAULT_NODE_URL.to_string(),
                wallet,
                run_fingerprints: true,
            };
            let mut miner = openfang_clawrtc::miner::Miner::new(config).expect("Miner init failed");
            let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

            let cancel_clone = cancel.clone();
            rt.block_on(async {
                // Spawn signal handler
                let cancel_sig = cancel_clone.clone();
                tokio::spawn(async move {
                    tokio::signal::ctrl_c().await.ok();
                    cancel_sig.store(true, std::sync::atomic::Ordering::Relaxed);
                });

                if let Err(e) = miner.mine_loop(cancel_clone).await {
                    eprintln!("{} {e}", "Mining error:".red());
                }
            });
        }
        Commands::Stop => {
            println!("Stopping miner (send SIGTERM to process)...");
        }
        Commands::Status => {
            rt.block_on(async {
                let client = openfang_clawrtc::RustChainClient::default_node();
                match client.health().await {
                    Ok(h) => {
                        println!("{} {}", "Node:".green(), if h.ok { "healthy" } else { "unhealthy" });
                        if let Some(v) = h.version {
                            println!("Version: {v}");
                        }
                    }
                    Err(e) => println!("{} {e}", "Error:".red()),
                }
            });
        }
        Commands::Wallet { action } => match action {
            WalletAction::Create { force } => {
                let path = dirs::home_dir()
                    .unwrap_or_default()
                    .join(".clawrtc/wallets/default.json");
                if path.exists() && !force {
                    eprintln!("Wallet already exists. Use --force to overwrite.");
                    std::process::exit(1);
                }
                let w = openfang_clawrtc::RtcWallet::generate();
                w.save_plaintext(&path).expect("Failed to save");
                println!("{} {}", "Address:".green(), w.address());
                println!("{} {}", "Public Key:".green(), w.public_key_hex());
                println!("Saved to: {}", path.display());
            }
            WalletAction::Show => {
                let path = dirs::home_dir()
                    .unwrap_or_default()
                    .join(".clawrtc/wallets/default.json");
                let w = openfang_clawrtc::RtcWallet::from_file(&path)
                    .expect("No wallet found. Run: clawrtc wallet create");
                println!("{} {}", "Address:".green(), w.address());
                println!("{} {}", "Public Key:".green(), w.public_key_hex());

                rt.block_on(async {
                    let client = openfang_clawrtc::RustChainClient::default_node();
                    match client.balance(w.address()).await {
                        Ok(bal) => println!("{} {} RTC", "Balance:".green(), bal),
                        Err(_) => println!("Balance: (offline)"),
                    }
                });
            }
            WalletAction::Export { output } => {
                let path = dirs::home_dir()
                    .unwrap_or_default()
                    .join(".clawrtc/wallets/default.json");
                let w = openfang_clawrtc::RtcWallet::from_file(&path)
                    .expect("No wallet found");
                let export = serde_json::json!({
                    "address": w.address(),
                    "public_key": w.public_key_hex(),
                });
                let json = serde_json::to_string_pretty(&export).unwrap();
                if let Some(out) = output {
                    std::fs::write(&out, &json).expect("Failed to write export");
                    println!("Exported to {out}");
                } else {
                    println!("{json}");
                }
            }
        },
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI feature not enabled. Build with: cargo build --features cli");
    std::process::exit(1);
}
