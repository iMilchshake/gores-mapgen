use std::{
    collections::HashMap,
    panic,
    path::{Path, PathBuf},
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
    ListConfigs(BridgeArgs),
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

    /// path to maps directory
    maps: PathBuf,

    /// path to base maps directory
    #[arg(default_value = "../data/maps")]
    base_maps: PathBuf,

    /// path to generation configurations directory
    #[arg(default_value = "../data/configs/gen")]
    gen_configs: PathBuf,

    /// path to map configurations directory
    #[arg(default_value = "../data/configs/map")]
    map_configs: PathBuf,

    /// how many times generation is retried
    #[arg(default_value_t = 10, long, short('r'))]
    generation_retries: usize,
}

/// keeps track of the server bridge state
pub struct ServerBridge {
    /// econ connection to game server
    econ: Option<Econ>,

    /// stores all available base maps paths
    base_maps: Vec<PathBuf>,

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
        let base_maps = load_base_maps_paths(args.base_maps.as_path());
        let gen_configs =
            load_configs_from_dir::<GenerationConfig, _>(args.gen_configs.as_path()).unwrap();
        let map_configs =
            load_configs_from_dir::<MapConfig, _>(args.map_configs.as_path()).unwrap();

        let current_map_config = map_configs.iter().last().unwrap().0.clone();
        let current_gen_config = gen_configs.iter().last().unwrap().0.clone();

        ServerBridge {
            econ: None,
            base_maps,
            gen_configs,
            map_configs,
            current_map_config,
            current_gen_config,
            args,
        }
    }

    fn start(&mut self) {
        self.econ = Some(
            Econ::connect(&format!("127.0.0.1:{}", self.args.port), 1024).unwrap_or_else(|error| {
                panic!("Failed to establish stream connection: {}", error);
            }),
        );

        if !self.econ_unchecked().is_authed() {
            info!(auth!("Trying to authenticate..."));

            let password = self.args.password.clone();

            if self.econ_unchecked().auth(&password) {
                info!(auth!("Authentication succeed"));
                info!(gen!("Generating initial map..."));

                let seed = Seed::random();
                let map_name = self.generate_map(seed, self.args.generation_retries);
                self.change_map(&map_name);
            } else {
                error!(auth!("Authentication failed, try another password"));
                panic!();
            }
        }

        loop {
            match self.econ_unchecked().read() {
                Ok(()) => {
                    while let Some(line) = &self.econ_unchecked().pop_line() {
                        if line.len() < 22 {
                            warn!(recv!("Incomplete econ line: {}"), line);
                            continue;
                        }

                        info!(recv!("{}"), &line[22..]);

                        self.check_call(line);
                    }
                }
                Err(err) => error!(recv!("{}"), err),
            }
        }
    }

    fn clear_votes(&mut self) {
        self.econ_unchecked().send_rcon_cmd("clear_votes").unwrap();
    }

    fn add_vote(&mut self, desc: &str, command: &str) {
        self.econ_unchecked()
            .send_rcon_cmd(&format!("add_vote \"{}\" \"{}\"", desc, command))
            .unwrap();
    }

    fn update_votes(&mut self) {
        self.clear_votes();

        let mut gap_size = 1;

        let mut gap = || {
            let gap = " ".repeat(gap_size);

            gap_size += 1;

            return gap;
        };

        self.add_vote(
            &format!("Random Map Generator by iMilchshake, v{}", crate_version!()),
            "info",
        );
        self.add_vote(&gap(), "info");

        self.add_vote(
            &format!("Current map configuration: {}", self.current_map_config),
            "info",
        );
        self.add_vote(
            &format!(
                "Current generator configuration: {}",
                self.current_gen_config
            ),
            "info",
        );
        self.add_vote(&gap(), "info");

        self.add_vote("Generate Random Map", "echo call generate");
        self.add_vote(&gap(), "info");

        let map_config_names_copy = self.map_configs.keys().map(String::clone).collect_vec();

        for map_config_name in map_config_names_copy {
            self.add_vote(
                &format!("Set map configuration: {}", &map_config_name),
                &format!("echo call configurate map {}", &map_config_name),
            );
        }

        self.add_vote(&gap(), "info");

        let gen_config_names_copy = self.gen_configs.keys().map(String::clone).collect_vec();

        for gen_config_name in gen_config_names_copy {
            self.add_vote(
                &format!("Set generator configuration: {}", &gen_config_name),
                &format!("echo call configurate gen {}", &gen_config_name),
            );
        }
    }

    /// checks whether the econ message regards votes
    fn check_call(&mut self, data: &str) {
        let mut callback_args = Vec::new();

        let mut idx = 0;

        for piece_view in data.split(' ') {
            if idx == 3 {
                // handle only echo
                if piece_view != "console:" {
                    return;
                }
            } else if idx == 4 {
                // handle only "call"
                if piece_view != "call" {
                    return;
                }
            } else if idx > 4 {
                callback_args.push(piece_view);
            }

            idx += 1;
        }

        match callback_args[0] {
            "generate" => {
                let seed = Seed::random();
                let map_name = self.generate_map(seed, self.args.generation_retries);
                self.change_map(&map_name);
            }
            "configurate" => match callback_args[1] {
                "gen" => {
                    if callback_args.len() < 3 {
                        warn!(gen!("Missing arguments on configuration call"));
                        return;
                    }

                    if !self.gen_configs.contains_key(callback_args[2]) {
                        warn!(
                            gen!("Unknown generator configuration: {}"),
                            callback_args[2]
                        );
                        return;
                    }

                    // TODO: quotation marks?
                    self.current_gen_config = callback_args[2].to_string();
                }
                "map" => {
                    if callback_args.len() < 3 {
                        warn!(gen!("Missing arguments on configuration call"));
                        return;
                    }

                    if !self.map_configs.contains_key(callback_args[2]) {
                        warn!(gen!("Unknown map configuration: {}"), callback_args[2]);
                        return;
                    }

                    // TODO: quotation marks?
                    self.current_map_config = callback_args[2].to_string();
                }
                s => warn!(gen!("Unknown configuration: {}"), s),
            },
            _ => {}
        }

        self.update_votes()
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

                let idx = Seed::random().0 as usize % self.base_maps.len();

                let base_map_path = &self.base_maps[idx];

                let mut tw_map =
                    TwMap::parse_file(base_map_path).expect("failed to parse base map");
                tw_map.load().expect("failed to load base map");

                let mut exporter = Exporter::new(&mut tw_map, &map, ExporterConfig::default());

                exporter.finalize().save_map(&map_path);

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
                self.say("GENERATION PANICKED, THIS SHOULD NOT HAPPEN");
                self.say("Please report this to iMilchshake, thanks :D");
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

    fn change_map(&mut self, map_name: &str) {
        self.econ_unchecked()
            .send_rcon_cmd(&format!("change_map {}", map_name))
            .unwrap();
        self.econ_unchecked().send_rcon_cmd("reload").unwrap();
    }

    pub fn say(&mut self, message: &str) {
        self.econ_unchecked()
            .send_rcon_cmd(&format!("say {message}"))
            .unwrap();
    }

    fn econ_unchecked(&mut self) -> &mut Econ {
        self.econ.as_mut().unwrap()
    }

    pub fn run() {
        match Command::parse() {
            Command::StartBridge(args) => ServerBridge::new(args).start(),
            Command::ListConfigs(args) => print_configs(args),
        }
    }
}

fn print_configs(args: BridgeArgs) {
    println!(
        "GenerationConfig: {}",
        load_configs_from_dir::<GenerationConfig, _>(args.gen_configs.as_path())
            .unwrap()
            .keys()
            .into_iter()
            .join(",")
    );
    println!(
        "MapConfig: {}",
        load_configs_from_dir::<MapConfig, _>(args.map_configs.as_path())
            .unwrap()
            .keys()
            .into_iter()
            .join(",")
    );
}

fn load_base_maps_paths<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for file_path in std::fs::read_dir(path).unwrap() {
        let file_path = file_path.unwrap().path();

        paths.push(file_path);
    }

    paths
}
