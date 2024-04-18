use clap::Parser;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
use gores_mapgen_rust::random::Random;
use gores_mapgen_rust::{config::GenerationConfig, generator::Generator};
use std::collections::HashMap;

use regex::Regex;
use std::{path::PathBuf, process::exit, str::FromStr, time::Duration};
use telnet::{Event, Telnet};

#[derive(Parser, Debug)]
struct BridgeArgs {
    /// ec_password
    econ_pass: String,

    /// ec_port
    econ_port: u16,

    /// telnet buffer size (amount of bytes/chars)
    #[arg(default_value_t = 256, long, short('b'))]
    telnet_buffer: usize,

    /// debug to console
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    /// path to maps folder
    maps: PathBuf,
}

#[derive(Parser, Debug)]
#[command(name = "DDNet Bridge")]
#[command(version = "0.1a")]
#[command(about = "Detect DDNet-Server votes via econ to trigger map generations", long_about = None)]
enum Command {
    /// Your new subcommand description here
    #[clap(name = "start", about = "Description of your new subcommand")]
    StartBridge(BridgeArgs),

    /// Your new subcommand description here
    #[clap(name = "presets", about = "Description of your new subcommand")]
    ListPresets,
}

#[derive(Debug)]
struct Vote {
    player_name: String,
    vote_name: String,
    vote_reason: String,
}

struct Econ {
    telnet: Telnet,
}

impl Econ {
    pub fn new(port: u16, buffer_size: usize) -> Econ {
        let address = SocketAddr::new(
            IpAddr::V4(Ipv4Addr::from_str("127.0.0.1").expect("Invalid address")),
            port,
        );

        Econ {
            telnet: Telnet::connect_timeout(&address, buffer_size, Duration::from_secs(10))
                .unwrap_or_else(|err| {
                    println!("Coulnt establish telnet connection\nError: {:?}", err);
                    exit(1);
                }),
        }
    }

    pub fn read(&mut self) -> Option<String> {
        // let event = self.telnet.read_nonblocking().expect("telnet read error");
        let event = self.telnet.read().expect("telnet read error");

        if let Event::Data(buffer) = event {
            Some(String::from_utf8_lossy(&buffer).replace('\0', ""))
        } else {
            None
        }
    }

    pub fn send_rcon_cmd(&mut self, mut command: String) {
        command.push('\n');
        self.telnet
            .write(command.as_bytes())
            .expect("telnet write error");
    }

    pub fn handle_vote(
        &mut self,
        vote: &Vote,
        args: &BridgeArgs,
        configs: &HashMap<String, GenerationConfig>,
    ) {
        if vote.vote_name.starts_with("generate") {
            // get user seed or random
            let seed = vote
                .vote_reason
                .parse::<u64>()
                .unwrap_or_else(|_| Random::get_random_seed());

            // split selected preset
            let mut vote_parts = vote.vote_name.split_whitespace();
            let vote_type = vote_parts.next().expect("should have exactly two parts");
            assert_eq!(vote_type, "generate");
            let vote_preset = vote_parts.next().expect("should have exactly two parts");
            assert!(vote_parts.next() == None, "should have exactly two parts");

            // get config based on preset name
            let config = configs.get(vote_preset).expect("preset does not exist!");

            generate_and_change_map(args, seed, config, self);
        }
    }
}

fn generate_and_change_map(
    args: &BridgeArgs,
    seed: u64,
    config: &GenerationConfig,
    econ: &mut Econ,
) {
    println!("[GEN] Starting Map Generation!");
    econ.send_rcon_cmd("say [GEN] Generating Map, seed=".to_string() + &seed.to_string());
    let map_path = args.maps.canonicalize().unwrap().join("random_map.map");
    match Generator::generate_map(30_000, seed, config) {
        Ok(map) => {
            println!("[GEN] Finished Map Generation!");
            map.export(&map_path);
            println!("[GEN] Map was exported");
            econ.send_rcon_cmd("change_map random_map".to_string());
            econ.send_rcon_cmd("reload".to_string());
            econ.send_rcon_cmd("say [GEN] Done...".to_string());
        }
        Err(err) => {
            println!("[GEN] Generation Error: {:?}", err);
            econ.send_rcon_cmd("say [GEN] Failed :( ".to_string());
        }
    }
}

fn main() {
    let cmd = Command::parse();

    if let Command::StartBridge(args) = cmd {
        // this regex detects all possible chat messages involving votes
        let vote_regex = Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I chat: \*\*\* (Vote passed|Vote failed|'(.+?)' called .+ option '(.+?)' \((.+?)\))\n").unwrap();
        let mut econ = Econ::new(args.econ_port, args.telnet_buffer);
        let mut pending_vote: Option<Vote> = None;
        let configs = GenerationConfig::get_configs();

        loop {
            if let Some(data) = econ.read() {
                if args.debug {
                    println!("[RECV DEBUG]: {:?}", data);
                }

                if data == "Enter password:\n" {
                    econ.send_rcon_cmd(args.econ_pass.clone());
                    println!("[AUTH] Sending login");
                } else if data.starts_with("Authentication successful") {
                    println!("[AUTH] Success");
                    println!("[GEN] Generating initial map");
                    generate_and_change_map(&args, 42, &GenerationConfig::default(), &mut econ);
                } else if data.starts_with("Wrong password") {
                    println!("[AUTH] Wrong Password!");
                    std::process::exit(1);
                } else {
                    let result = vote_regex.captures_iter(&data);

                    for mat in result {
                        let _date = mat.get(1).unwrap();
                        let message = mat.get(2);

                        // determine vote event type
                        if let Some(message) = message.map(|v| v.as_str()) {
                            match message {
                                "Vote passed" => {
                                    println!("[VOTE]: Success");
                                    econ.handle_vote(
                                        pending_vote.as_ref().unwrap(),
                                        &args,
                                        &configs,
                                    );
                                }
                                "Vote failed" => {
                                    pending_vote = None;
                                    println!("[VOTE]: Failed");
                                }
                                // vote started messages begin with 'player_name'
                                _ if message.starts_with('\'') => {
                                    let player_name = mat.get(3).unwrap().as_str().to_string();
                                    let vote_name = mat.get(4).unwrap().as_str().to_string();
                                    let vote_reason = mat.get(5).unwrap().as_str().to_string();

                                    println!(
                                        "[VOTE]: vote_name={}, vote_reason={}, player={}",
                                        &vote_name, &vote_reason, &player_name
                                    );

                                    pending_vote = Some(Vote {
                                        player_name,
                                        vote_name,
                                        vote_reason,
                                    });
                                }
                                // panic if for some holy reason something else matched the regex
                                _ => panic!(),
                            }
                        }
                    }
                }
            }
        }
    } else {
        // yeah i know that this is really badly organized, dont shame me. this will improve!
        let configs = GenerationConfig::get_configs();
        for preset in configs.keys() {
            println!("{}", preset);
        }
    }
}
