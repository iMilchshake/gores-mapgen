use std::{
    collections::HashMap,
    io::{Error, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    panic,
    path::PathBuf,
    time::Duration,
};

use mapgen_core::{
    config::GenerationConfig,
    config::{load_configs_from_dir, MapConfig},
    generator::Generator,
    random::Seed,
};
use mapgen_exporter::{Exporter, ExporterConfig};

use clap::{crate_version, Parser};
use itertools::Itertools;
use log::{error, info, warn};
use regex::Regex;
use simple_logger::SimpleLogger;
use twmap::TwMap;

#[derive(Parser, Debug)]
#[command(name = "DDNet Bridge")]
#[command(version = crate_version!())]
#[command(about = "Detect DDNet-Server votes via econ to trigger map generations", long_about = None)]
enum Command {
    #[clap(name = "start", about = "Start the ddnet bridge")]
    StartBridge(BridgeArgs),

    #[clap(
        name = "list",
        about = "Print a list of available map- & generation configs"
    )]
    ListConfigs,
}

macro_rules! auth {
    ($e:expr) => {
        concat!("\x1b[38;5;85mAUTH\x1b[0m", " ", $e, "\x1b[0m")
    };
}

macro_rules! recv {
    ($e:expr) => {
        concat!("\x1b[38;5;94mRECV\x1b[0m\x1b[90m", " ", $e, "\x1b[0m")
    };
}

macro_rules! gen {
    ($e:expr) => {
        concat!("\x1b[38;5;183mGEN \x1b[0m", " ", $e, "\x1b[0m")
    };
}

macro_rules! vote {
    ($e:expr) => {
        concat!("\x1b[38;5;95mVOTE\x1b[0m", " ", $e, "\x1b[0m")
    };
}

#[derive(Parser, Debug)]
struct BridgeArgs {
    /// ec_password
    password: String,

    /// ec_port
    port: u16,

    /// telnet buffer size (amount of bytes/chars)
    #[arg(default_value_t = 256, long, short('b'))]
    buffer_size: usize,

    /// debug to console
    #[arg(short, long, default_value_t = false)]
    debug: bool,

    /// path to maps folder
    maps: PathBuf,

    /// how many times generation is retried
    #[arg(default_value_t = 10, long, short('r'))]
    generation_retries: usize,
}

#[derive(Debug)]
struct Vote {
    _player_name: String,
    vote_name: String,
    vote_reason: String,
}

struct Econ {
    connection: TcpStream,
    buffer: Vec<u8>,
    lines: Vec<String>,
    unfinished_line: String,
    authed: bool,
}

impl Econ {
    pub fn connect<A: ToSocketAddrs>(address: A, buffer_size: usize) -> Result<Self, Error> {
        let address = address.to_socket_addrs().unwrap().next().unwrap();

        let connection = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;

        Ok(Self {
            connection,
            buffer: vec![0; buffer_size],
            lines: Vec::new(),
            unfinished_line: String::new(),
            authed: false,
        })
    }

    pub fn read(&mut self) -> Result<(), Error> {
        let written = self.connection.read(&mut self.buffer)?;

        if written != 0 {
            let mut lines = String::from_utf8_lossy(&self.buffer[..written])
                .replace('\0', "")
                .split("\n")
                .map(String::from)
                .collect_vec();

            if lines.last().unwrap() == "" {
                let _ = lines.pop();

                if !self.unfinished_line.is_empty() {
                    let take = self.unfinished_line.to_owned();
                    lines[0] = take + &lines[0];

                    self.unfinished_line.clear();
                }
            } else {
                self.unfinished_line = lines.pop().unwrap();
            }

            self.lines.extend(lines);
        }

        Ok(())
    }

    pub fn pop_line(&mut self) -> Option<String> {
        self.lines.pop()
    }

    pub fn send_rcon_cmd(&mut self, command: &str) {
        self.connection
            .write(command.as_bytes())
            .expect("stream write error");
        self.connection
            .write("\n".as_bytes())
            .expect("stream write error");

        self.connection.flush().expect("stream flush error");
    }

    pub fn rcon_say(&mut self, message: &str) {
        self.send_rcon_cmd(&format!("say {message}"));
    }
}

/// keeps track of the server bridge state
struct ServerBridge {
    /// econ connection to game server
    econ: Option<Econ>,

    /// stores information about vote while its still pending
    pending_vote: Option<Vote>,

    /// stores all available generation configs
    gen_configs: HashMap<String, GenerationConfig>,

    /// stores all available map configs
    map_configs: HashMap<String, MapConfig>,

    /// selected map config
    current_map_config: String,

    /// selected gen config
    current_gen_config: String,

    /// stores start arguments
    args: BridgeArgs,
}

impl ServerBridge {
    fn new(args: BridgeArgs) -> ServerBridge {
        let gen_configs =
            load_configs_from_dir::<GenerationConfig, _>("../data/configs/gen").unwrap();
        let map_configs = load_configs_from_dir::<MapConfig, _>("../data/configs/map").unwrap();

        let current_map_config = map_configs.iter().last().unwrap().0.clone();
        let current_gen_config = gen_configs.iter().last().unwrap().0.clone();

        ServerBridge {
            econ: None,
            pending_vote: None,
            gen_configs,
            map_configs,
            current_map_config,
            current_gen_config,
            args,
        }
    }

    fn start(&mut self) {
        self.econ = Some(
            Econ::connect(
                &format!("127.0.0.1:{}", self.args.port),
                self.args.buffer_size,
            )
            .unwrap_or_else(|error| {
                panic!("Failed to establish stream connection: {}", error);
            }),
        );

        if self.econ.is_some() {
            loop {
                match self.econ_unchecked().read() {
                    Ok(()) => {
                        while let Some(line) = &self.econ_unchecked().pop_line() {
                            if !self.econ_unchecked().authed {
                                self.check_auth(line);
                            } else {
                                if line.len() < 22 {
                                    warn!(recv!("Incomplete econ line: {}"), line);
                                } else {
                                    info!(recv!("{}"), &line[22..]);
                                }

                                self.check_vote(line);
                            }
                        }
                    }
                    Err(error) => error!(recv!("{}"), error),
                }
            }
        }
    }

    /// checks whether the econ message regards votes
    pub fn check_vote(&mut self, data: &String) {
        // this regex detects all possible chat messages involving votes
        let vote_regex = Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I chat: \*\*\* (Vote passed.*|Vote failed|'(.+?)' called .+ option '(.+?)' \((.+?)\))").unwrap();
        let result = vote_regex.captures_iter(&data);

        for mat in result {
            let _date = mat.get(1).unwrap();
            let message = mat.get(2);

            // determine vote event type
            if let Some(message) = message.map(|v| v.as_str()) {
                match message {
                    _ if message.starts_with("Vote passed") => {
                        info!(vote!("Success"));
                        self.handle_pending_vote();
                    }
                    "Vote failed" => {
                        self.pending_vote = None;
                        info!(vote!("Failed"));
                    }
                    // vote started messages begin with 'player_name'
                    _ if message.starts_with('\'') => {
                        let player_name = mat.get(3).unwrap().as_str().to_string();
                        let vote_name = mat.get(4).unwrap().as_str().to_string();
                        let vote_reason = mat.get(5).unwrap().as_str().to_string();

                        info!(
                            vote!("vote_name={}, vote_reason={}, player={}"),
                            &vote_name, &vote_reason, &player_name
                        );

                        self.pending_vote = Some(Vote {
                            _player_name: player_name,
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

    /// checks whether the econ message regards authentication
    pub fn check_auth(&mut self, data: &String) {
        let mut generate = false;
        let password = &self.args.password;

        if let Some(econ) = &mut self.econ {
            if data == "Enter password:" {
                info!(auth!("Sending login"));

                econ.send_rcon_cmd(password);
            } else if data.starts_with("Authentication successful") {
                info!(auth!("Success"));
                info!(gen!("Generating initial map"));

                econ.authed = true;
                generate = true;
            } else if data.starts_with("Wrong password") {
                panic!("Failed to connect to game server: wrong password");
            }
        }

        if generate {
            self.generate_and_change_map(Seed::from_u64(1337), self.args.generation_retries);
        }
    }

    pub fn handle_pending_vote(&mut self) {
        if let Some(vote) = self.pending_vote.take() {
            if vote.vote_name.to_lowercase().starts_with("generate") {
                // derive Seed from vote reason
                let seed = if vote.vote_reason == "No reason given" {
                    Seed::random()
                } else if let Ok(seed_u64) = vote.vote_reason.parse::<u64>() {
                    Seed::from_u64(seed_u64)
                } else {
                    Seed::from_str(&vote.vote_reason)
                };

                // split vote name to get selected preset
                let config_name = vote
                    .vote_name
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap();

                self.current_gen_config = config_name.to_owned();

                self.generate_and_change_map(seed, self.args.generation_retries);
            } else if vote.vote_name.starts_with("change_layout") {
                // split vote name to get selected preset
                let config_name = vote
                    .vote_name
                    .splitn(2, char::is_whitespace)
                    .nth(1)
                    .unwrap();

                info!(gen!("changed layout to {}"), config_name);

                // overwrite current map config
                self.current_map_config = config_name.to_owned();
            }
        } else {
            warn!(vote!(
                "Vote succeed, but no pending vote! unhandled vote type?"
            ));
        }
    }

    fn generate_and_change_map(&mut self, seed: Seed, retries: usize) {
        let map_path = self
            .args
            .maps
            .canonicalize()
            .unwrap()
            .join("random_map.map");

        info!(
            gen!("Generating | seed={:?} | gen_cfg={:?} | map_cfg={:?}"),
            &seed, &self.current_gen_config, &self.current_map_config
        );

        let gen_status = panic::catch_unwind(|| {
            Generator::generate_map(
                100_000,
                seed,
                self.gen_configs[&self.current_gen_config].clone(),
                self.map_configs[&self.current_map_config].clone(),
            )
        });

        match gen_status {
            // map was generated successfully
            Ok(Ok(map)) => {
                info!(gen!("Finished Map Generation!"));

                let mut tw_map =
                    TwMap::parse_file("automap_test.map").expect("failed to parse base map");
                tw_map.load().expect("failed to load base map");

                let mut exporter = Exporter::new(&mut tw_map, &map, ExporterConfig::default());

                exporter.finalize(&map_path);

                info!(gen!("Map was exported"));
                self.econ_unchecked().send_rcon_cmd("change_map random_map");
                self.econ_unchecked().send_rcon_cmd("reload");
            }
            // map generation failed -> just retry
            Ok(Err(generation_error)) => {
                warn!(gen!("Generation Error: {:?}"), generation_error);

                if retries > 0 {
                    // retry with different seed
                    let mut seed = seed;
                    seed.0 = seed.0.wrapping_add(1);
                    self.generate_and_change_map(seed, retries - 1);
                }
            }
            // map generation panic -> STOP
            Err(panic_info) => {
                error!(gen!("Generation panicked!"));
                error!(gen!("{:?}"), panic_info);
                self.econ_unchecked()
                    .rcon_say("GENERATION PANICKED, THIS SHOULD NOT HAPPEN");
                self.econ_unchecked()
                    .rcon_say("Please report this to iMilchshake, thanks :D");
            }
        }
    }

    fn econ_unchecked(&mut self) -> &mut Econ {
        self.econ.as_mut().unwrap()
    }
}

fn print_configs() {
    println!(
        "GenerationConfig: {}",
        load_configs_from_dir::<GenerationConfig, _>("../data/configs/gen")
            .unwrap()
            .keys()
            .into_iter()
            .join(",")
    );
    println!(
        "MapConfig: {}",
        load_configs_from_dir::<MapConfig, _>("../data/configs/map")
            .unwrap()
            .keys()
            .into_iter()
            .join(",")
    );
}

fn main() {
    match Command::parse() {
        Command::StartBridge(bridge_args) => {
            SimpleLogger::new().init().unwrap();
            let mut bridge = ServerBridge::new(bridge_args);
            bridge.start();
        }
        Command::ListConfigs => print_configs(),
    }
}
