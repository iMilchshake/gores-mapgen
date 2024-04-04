use regex::Regex;
use std::{thread::sleep, time::Duration};
use telnet::{Event, Telnet};

const EXPORT: bool = false;

#[derive(Debug)]
struct Vote {
    player_name: String,
    vote_name: String,
    vote_reason: String,
}

impl Vote {
    pub fn handle(&self) {
        if self.vote_name == "generate" {
            println!("[DEBUG] Generating Map...")
        }
    }
}

fn main() {
    // this regex detects all possible chat messages involving votes
    let vote_regex = Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I chat: \*\*\* (Vote passed|Vote failed|'(.+?)' called .+ option '(.+?)' \((.+?)\))\n").unwrap();

    // ddnet econ connection using telnet
    let mut econ = Telnet::connect(("localhost", 16321), 256).expect("cant connect to econ!");

    let mut pending_vote: Option<Vote> = None;

    loop {
        let event = econ.read_nonblocking().expect("telnet read error");

        if let Event::Data(buffer) = event {
            let ascii_data = String::from_utf8_lossy(&buffer);
            let ascii_data = ascii_data.replace("\0", "");

            #[cfg(EXPORT)]
            println!("[RECV DEBUG]: {:?}", ascii_data);

            if ascii_data == "Enter password:\n" {
                let password = "a\n".as_bytes();
                econ.write(&password).expect("write error");
                println!("[AUTH] Sending login");
            } else if ascii_data.starts_with("Authentication successful") {
                println!("[AUTH] Success");
            } else {
                let result = vote_regex.captures_iter(&ascii_data);

                for mat in result {
                    let _date = mat.get(1).unwrap();
                    let message = mat.get(2);

                    // determine vote event type
                    if let Some(message) = message.map(|v| v.as_str()) {
                        match message {
                            "Vote passed" => {
                                println!("[VOTE]: Success");
                                pending_vote.as_ref().unwrap().handle();
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

        sleep(Duration::from_secs_f32(0.01));

        // Do something else ...
    }
}

// this matches server and client
//  (\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}) I (chat|server): (?:ClientID=(\d) |(\d):(-?\d):(.+?):(.+?))(.+?)\\n
