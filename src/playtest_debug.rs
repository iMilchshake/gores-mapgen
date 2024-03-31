use std::env;
use std::fs;
use std::{path::PathBuf, process::Command};

use crate::map::Map;

/// This is a highly WIP implementation for instant playtest inside the DDNet client. Use with
/// caution..
#[derive(Debug)]
pub struct PlaytestDebug {
    ddnet: String,
    ddnet_server: String,
    maps: PathBuf,
}

impl PlaytestDebug {
    pub fn new() -> PlaytestDebug {
        PlaytestDebug {
            ddnet: "DDNet".to_string(),
            ddnet_server: "DDNet-Server".to_string(),
            maps: PathBuf::from("/home/tobi/.local/share/ddnet/maps"),
        }
    }

    pub fn playtest(&self, map: &Map) {
        let cwd = env::current_dir().unwrap();
        map.export("playtest".to_string());
        let map_path = cwd.join("playtest.map");
        let map_destination = self.maps.join("playtest.map");
        dbg!(&map_path, &map_destination);

        // copy playtest map to maps folder
        if let Err(err) = fs::copy(&map_path, &map_destination) {
            dbg!(err);
        } else {
            // kill remaining servers
            let _ = Command::new("pkill").arg("DDNet-Server").status();

            let mut server = Command::new(&self.ddnet_server)
                .args(["change_map playtest"])
                .spawn()
                .expect("ddnet server failed");

            // run ddnet client blocking, so we can easily check whether the game was closed
            let _ = Command::new(&self.ddnet)
                .args(["connect localhost"])
                .status();

            // afterwards kill the server process
            server
                .kill()
                .expect("coulnt kill server, it might still run..");
        }
    }
}
