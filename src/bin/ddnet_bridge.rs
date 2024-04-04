use regex::Regex;
use std::{thread::sleep, time::Duration};
use telnet::{Event, Telnet};

const EXPORT: bool = false;
const TELNET_BUFFER: usize = 256;
const TELNET_DELAY: f32 = 1.0;

#[derive(Debug)]
struct Vote {
    player_name: String,
    vote_name: String,
    vote_reason: String,
}

impl Vote {}

struct Econ {
    telnet: Telnet,
}

impl Econ {
    pub fn new(port: u16) -> Econ {
        Econ {
            telnet: Telnet::connect(("localhost", port), TELNET_BUFFER)
                .expect("cant connect to econ!"),
        }
    }

    pub fn read(&mut self) -> Option<String> {
        let event = self.telnet.read_nonblocking().expect("telnet read error");

        if let Event::Data(buffer) = event {
            Some(String::from_utf8_lossy(&buffer).replace("\0", ""))
        } else {
            None
        }
    }

    pub fn send_command(&mut self, mut command: String) {
        command.push('\n');
        self.telnet
            .write(command.as_bytes())
            .expect("telnet write error");
    }

    pub fn handle_vote(&mut self, vote: &Vote) {
        if vote.vote_name == "generate" {
            println!("[DEBUG] Generating Map...");

            // TODO: Actually generate map here ...

            self.send_command("say [DEBUG] Generating Map...".to_string());
        }
    }
}

fn main() {
    // this regex detects all possible chat messages involving votes
    let vote_regex = Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I chat: \*\*\* (Vote passed|Vote failed|'(.+?)' called .+ option '(.+?)' \((.+?)\))\n").unwrap();
    let mut econ = Econ::new(16321);
    let mut pending_vote: Option<Vote> = None;

    loop {
        if let Some(data) = econ.read() {
            #[cfg(EXPORT)]
            println!("[RECV DEBUG]: {:?}", ascii_data);

            if data == "Enter password:\n" {
                econ.send_command("a".to_string()); // TODO: actually add password
                println!("[AUTH] Sending login");
            } else if data.starts_with("Authentication successful") {
                println!("[AUTH] Success");
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
                                econ.handle_vote(pending_vote.as_ref().unwrap());
                            }
                            "Vote failed" => {
                                pending_vote = None;
                                println!("[VOTE]: Failed");
                            }
                            // vote started messages begin with 'player_name'
                            _ if message.starts_with("'") => {
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
                            _ => panic!(),
                        }
                    }
                }
            }
        }

        sleep(Duration::from_secs_f32(TELNET_DELAY));
    }
}

// this matches server and client
//  (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I (chat|server): (?:ClientID=(\d) |(\d):(-?\d):(.+?):(.+?))(.+?)\\n
