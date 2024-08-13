use std::collections::HashMap;
use std::panic;
use gores_mapgen::config::{GenerationConfig, MapConfig};
use gores_mapgen::generator::Generator;
use gores_mapgen::random::Seed;
use rand::prelude::*;
use std::time::Instant;


fn main() {

    //thanks tobi for the code hehehe
    let init_gen_configs: HashMap<String, GenerationConfig> = GenerationConfig::get_all_configs();
    let init_map_configs: HashMap<String, MapConfig> = MapConfig::get_all_configs();

    let seed: u64 = random::<u64>();
   
    for gkey  in init_gen_configs.keys(){
        for mkey in init_map_configs.keys(){

            let seed = Seed::from_u64(seed);
            let gen_cfg = init_gen_configs.get(gkey).unwrap();
            let map_cfg = init_map_configs.get(mkey).unwrap();

            let now = Instant::now();
            let _ = panic::catch_unwind(|| {
                let gen_result =  Generator::generate_map(
                    200_000,
                    &seed,
                    gen_cfg,
                    map_cfg
                );
                let elapsed = now.elapsed();
                match gen_result {
                    Ok(_) => println!("GEN {gkey} WITH {mkey} ELAPSED TIME: {elapsed:?\n}"),
                    Err(e) => println!("{e}"),
                }
            });
        }
    }
}

//MAKE GENERATION MAP OVER EVERY  GEN CFG & MAP CFG | MARK: DONE 
//RNG SEED | MARK: DONE
//TIME GENERATION  | MARK: DONE 
//PRINT TO UI | MARK: NOT STARTED