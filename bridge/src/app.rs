use std::{collections::HashMap, panic, path::PathBuf};

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

use crate::econ::*;

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

#[derive(Parser, Debug)]
struct BridgeArgs {
    /// ec_password
    password: String,

    /// ec_port
    port: u16,

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

/// keeps track of the server bridge state
pub struct ServerBridge<const BUFFER_SIZE: usize> {
    /// econ connection to game server
    econ: Option<Econ<BUFFER_SIZE>>,

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

impl<const BUFFER_SIZE: usize> ServerBridge<BUFFER_SIZE> {
    fn new(args: BridgeArgs) -> ServerBridge<BUFFER_SIZE> {
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
            Econ::connect(&format!("127.0.0.1:{}", self.args.port)).unwrap_or_else(|error| {
                panic!("Failed to establish stream connection: {}", error);
            }),
        );

        if self.econ.is_some() {
            loop {
                if !self.econ_unchecked().is_authed() {
                    info!(auth!("Trying to authenticate..."));
                    if self.check_auth() {
                        info!(gen!("Generating initial map..."));

                        let map_name =
                            self.generate_map(Seed::from_u64(1337), self.args.generation_retries);

                        self.econ_unchecked()
                            .send_rcon_cmd(&format!("change_map {}", &map_name))
                            .unwrap();
                        self.econ_unchecked().send_rcon_cmd("reload").unwrap();
                    } else {
                        error!(auth!("Authentication failed, try another password"));
                        panic!();
                    }
                }

                let result = self.econ_unchecked().read();
                if result.is_ok() {
                    result.unwrap();
                    while let Some(line) = &self.econ_unchecked().pop_line() {
                        if line.len() < 22 {
                            warn!(recv!("Incomplete econ line: {}"), line);
                        } else {
                            info!(recv!("{}"), &line[22..]);
                        }

                        self.check_vote(line);
                    }
                } else {
                    error!(recv!("{}"), result.unwrap_err());
                }
            }
        }
    }

    /// checks whether the econ message regards votes
    fn check_vote(&mut self, data: &String) {
        // this regex detects all possible chat messages involving votes
        let vote_regex = Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I chat: \*\*\* (Vote passed.*|Vote failed|'(.+?)' called .+ option '(.+?)' \((.+?)\))").unwrap();
        let result = vote_regex.captures_iter(&data);

        for mat in result {
            let _date = mat.get(1).unwrap();
            let message = mat.get(2);

            // determine vote event type
            if let Some(message) = message.map(|v| v.as_str()) {
                match message {
                    "Vote failed" => {
                        self.pending_vote = None;
                        info!(vote!("Failed"));
                    }
                    _ => {
                        if message.starts_with("Vote passed") {
                            info!(vote!("Success"));
                            self.handle_pending_vote();
                        } else if message.starts_with('\'') {
                            // vote started messages begin with 'player_name'
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
                        } else {
                            // panic if for some holy reason something else matched the regex
                            panic!();
                        }
                    }
                }
            }
        }
    }

    /// checks whether the econ message regards authentication
    fn check_auth(&mut self) -> bool {
        let password = &self.args.password;

        if let Some(econ) = &mut self.econ {
            assert!(econ.is_authed() == false);

            return econ.auth(password);
        }

        false
    }

    fn handle_pending_vote(&mut self) {
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

                let map_name = self.generate_map(seed, self.args.generation_retries);

                self.econ_unchecked()
                    .send_rcon_cmd(&format!("change_map {}", &map_name))
                    .unwrap();
                self.econ_unchecked().send_rcon_cmd("reload").unwrap();
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

    fn generate_map(&mut self, seed: Seed, retries: usize) -> String {
        let map_name = format!(
            "{}_{}_{}",
            &self.current_gen_config, &self.current_map_config, seed.0
        );

        let map_path = self
            .args
            .maps
            .canonicalize()
            .unwrap()
            .join(map_name.clone() + ".map");

        info!(
            gen!("Generating | seed={} | gen_cfg={} | map_cfg={}"),
            seed.0, &self.current_gen_config, &self.current_map_config
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
                info!(gen!("Finished map generation"));

                let mut tw_map =
                    TwMap::parse_file("automap_test.map").expect("failed to parse base map");
                tw_map.load().expect("failed to load base map");

                let mut exporter = Exporter::new(&mut tw_map, &map, ExporterConfig::default());

                exporter.finalize(&map_path);

                info!(gen!("Generated map was exported: {}"), &map_name);

                return map_name;
            }
            // map generation failed -> just retry
            Ok(Err(generation_error)) => {
                warn!(gen!("Generation Error: {:?}"), generation_error);
            }
            // map generation panic -> retryyy
            Err(panic_info) => {
                error!(gen!("Generation panicked!"));
                error!(gen!("{:?}"), panic_info);
                self.econ_unchecked()
                    .rcon_say("GENERATION PANICKED, THIS SHOULD NOT HAPPEN")
                    .unwrap();
                self.econ_unchecked()
                    .rcon_say("Please report this to iMilchshake, thanks :D")
                    .unwrap();
            }
        }

        // retry with different seed
        if retries > 0 {
            return self.generate_map(Seed::random(), retries - 1);
        }

        error!(gen!(
            "Failed to generate map after numerous retries. Give up"
        ));
        panic!()
    }

    fn econ_unchecked(&mut self) -> &mut Econ<BUFFER_SIZE> {
        self.econ.as_mut().unwrap()
    }

    pub fn run() {
        match Command::parse() {
            Command::StartBridge(bridge_args) => {
                SimpleLogger::new().init().unwrap();
                let mut bridge = ServerBridge::<512>::new(bridge_args);
                bridge.start();
            }
            Command::ListConfigs => print_configs(),
        }
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
