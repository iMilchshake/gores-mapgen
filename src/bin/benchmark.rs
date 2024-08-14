use gores_mapgen::config::{GenerationConfig, MapConfig};
use gores_mapgen::generator::Generator;
use gores_mapgen::random::Seed;
use rand::prelude::*;
use std::collections::HashMap;
use std::panic;
use std::time::Instant;

fn main() {
    //thanks tobi for the code hehehe
    let init_gen_configs: HashMap<String, GenerationConfig> = GenerationConfig::get_all_configs();
    let init_map_configs: HashMap<String, MapConfig> = MapConfig::get_all_configs();

    let seed: u64 = random::<u64>();
    //Loop through the keys of the hashmap
    for gkey in init_gen_configs.keys() {
        for mkey in init_map_configs.keys() {
            //We access the generate_map arguments now so the time from instant is only timing the generation function and not retreiving the data for the var also.
            let seed = Seed::from_u64(seed);
            let gen_cfg = init_gen_configs.get(gkey).unwrap();
            let map_cfg = init_map_configs.get(mkey).unwrap();

            let now = Instant::now();
            let _ = panic::catch_unwind(|| {
                let gen_result = Generator::generate_map(200_000, &seed, gen_cfg, map_cfg);
                let elapsed = now.elapsed(); //compare the time difference
                match gen_result {
                    //Handiling the Result<t,e>
                    Ok(_) => {
                        println!("GEN {gkey} WITH {mkey} MAP GEN | ELAPSED TIME: {elapsed:?\n}")
                    }
                    Err(e) => println!(
                        "ERROR IN GENERATING MAP: {e} | GENERATION THAT FAILED: {gkey} with {mkey}"
                    ),
                }
            });
        }
    }
}
