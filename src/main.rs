use clap::Parser;
use std::fs::{self, File};
use std::io::Write;
use std::time::Instant;
use rand::seq::SliceRandom;
mod bot;
mod mission;
mod wavespawn;
use crate::bot::Bot;
use crate::mission::Mission;
use crate::wavespawn::Wavespawn;

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    map: String,
    #[arg(short, long)]
    name: String,
    #[arg(short, long)]
    start_money: i32,
    #[arg(short, long)]
    config: String,
}
//Hierarchy: Mission -> Waves -> Wavespawns -> Bots
//Test: cargo run -- -m mvm_decoy -n lol -s 10000 -c normal_if.json
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut mission: Mission = Mission {
        ..Default::default()
    };
    let mut bots: Vec<Bot> = vec![];
    let mut wavespawns: Vec<Wavespawn> = vec![];

    mission.parse_mission_config(&args.config);
    mission.parse_map_config(&args.map);

    //free up memory on exit scope
    {
        let now = Instant::now();

        let bot_config = fs::read_to_string("./config/bots.json")?;
        let bot_info_string: serde_json::Value = serde_json::from_str(&bot_config)?;
        let bot_infos = &bot_info_string.as_object().unwrap();
        for value in *bot_infos {
            let new_bot: Bot = Bot {
                name: value.0.to_string(),
                class: match value.1["class"].as_str() {
                    None => "scout".to_string(),
                    Some(val) => val.to_string(),
                },
                class_icon: match value.1["class_icon"].as_str() {
                    None => "".to_string(),
                    Some(val) => val.to_string(),
                },
                weapons: match value.1["weapons"].as_array() {
                    None => vec![],
                    Some(val) => val.iter().map(|x| x.as_str().unwrap().to_owned()).collect(),
                },
                difficulty: match value.1["difficulty"].as_i64() {
                    None => 1,
                    Some(val) => val,
                },
                weapon_restriction: match value.1["weapon_restriction"].as_str() {
                    None => "".to_string(),
                    Some(val) => val.to_string(),
                },
                behavior: match value.1["behavior"].as_str() {
                    None => "".to_string(),
                    Some(val) => val.to_string(),
                },
                bot_attributes: match value.1["bot_attributes"].as_array() {
                    None => vec![],
                    Some(val) => val.iter().map(|x| x.as_str().unwrap().to_owned()).collect(),
                },
                health: match value.1["health"].as_str() {
                    None => "".to_string(),
                    Some(val) => val.to_string(),
                },
                scale: match value.1["scale"].as_f64() {
                    None => 1.0,
                    Some(val) => val,
                },
                max_vision_range: match value.1["max_vision_range"].as_i64() {
                    None => 0,
                    Some(val) => val,
                },
                auto_jump_min: match value.1["auto_jump_min"].as_i64() {
                    None => 0,
                    Some(val) => val,
                },
                auto_jump_max: match value.1["auto_jump_max"].as_i64() {
                    None => 0,
                    Some(val) => val,
                },
                is_boss: match value.1["is_boss"].as_bool() {
                    None => false,
                    Some(val) => val,
                },
                is_giant: match value.1["is_giant"].as_bool() {
                    None => false,
                    Some(val) => val,
                },
                currency_weight: match value.1["currency_weight"].as_i64() {
                    None => 1,
                    Some(val) => val,
                },
                count: match value.1["count"].as_i64() {
                    None => 10,
                    Some(val) => val,
                },
                max_active: match value.1["max_active"].as_i64() {
                    None => 10,
                    Some(val) => val,
                },
                spawn_per_timer: match value.1["spawn_per_timer"].as_i64() {
                    None => 2,
                    Some(val) => val,
                },
                time_before_spawn: match value.1["time_before_spawn"].as_i64() {
                    None => 0,
                    Some(val) => val,
                },
                time_between_spawn: match value.1["time_between_spawn"].as_i64() {
                    None => 5,
                    Some(val) => val,
                },
                attributes: match value.1["attributes"].as_array() {
                    None => vec![],
                    Some(val) => val
                        .iter()
                        .map(|x| match x.as_array() {
                            Some(inner) => [
                                inner[0].as_str().unwrap().to_owned(),
                                inner[1].as_str().unwrap().to_owned(),
                            ],
                            None => panic!("WTF! - Failed to parse attributes for {}", value.0.to_string()),
                        })
                        .collect(),
                }
            };
            bots.push(new_bot);
        }
        println!("took {:?} to parse bot config", now.elapsed());
    }
    {
        let now = Instant::now();

        let bot_config = fs::read_to_string("./config/wavespawns.json")?;
        let squad_info_string: serde_json::Value = serde_json::from_str(&bot_config)?;
        let squad_infos = &squad_info_string.as_object().unwrap();
        for value in *squad_infos {
            let wavespawn: Wavespawn = Wavespawn {
                squads: match value.1["squads"].as_array() {
                    None => vec![],
                    Some(val) => val.iter().map(|x| bots.iter().find(|y| *y.name == x.as_str().unwrap().to_owned()).unwrap() ).cloned().collect(),
                },
                tags: match value.1["tags"].as_array() {
                    None => vec![],
                    Some(val) => val.iter().map(|x| x.as_str().unwrap().to_owned()).collect(),
                },
                weight: match value.1["weight"].as_i64() {
                    None => 1,
                    Some(val) => val,
                },
                rarity: match value.1["rarity"].as_i64() {
                    None => 1,
                    Some(val) => val,
                },
            };
            for tag in &mission.wavespawn_tags{
                if wavespawn.tags.contains(tag){
                    wavespawns.push(wavespawn);
                    break;
                }
            }
        }
        println!("took {:?} to parse wavespawn config", now.elapsed());
    }
    //Wave Generation Process
    let mut pop_file = String::new();
    let mut output_file = File::create("./output/".to_string()+&args.map+"_"+&mission.mission_name+".pop")?;

    //Write the header of the file
    pop_file.push_str("//This file was generated by https://github.com/kurwabomber/mvm_generator\n");
    pop_file.push_str("#base robot_giant.pop\n#base robot_standard.pop\n#base robot_gatebot.pop\n");
    pop_file.push_str("population\n{\n");
    pop_file.push_str(&format!("\tStartingCurrency\t{}\n", mission.starting_money));
    pop_file.push_str("\tCanBotsAttackWhileInSpawnRoom\tNo\n");
    pop_file.push_str("\tFixedRespawnWaveTime\tYes\n");
    for i in 1..mission.wave_amount+1{
        println!("Wave {}", i);
        pop_file.push_str("\tWave\n\t{\n");
        pop_file.push_str("\t\tWaitWhenDone\t65\n");
        pop_file.push_str("\t\tCheckpoint\tYes\n");
        pop_file.push_str("\t\tStartWaveOutput{\n\t\t\tTarget	wave_start_relay\n\t\t\tAction	Trigger\n\t\t}\n");
        for squad_num in 1..mission.wavespawn_amount+1{
            println!("Generating Squad #{}", squad_num);
            let chosen_wavespawn = wavespawns.choose_weighted(&mut rand::thread_rng(), |item| item.weight).unwrap();
            for chosen_bot in &chosen_wavespawn.squads{
                println!("Contains {}", chosen_bot.name);
            }
        }
        pop_file.push_str("\t}\n");
    }
    pop_file.push_str("}\n");
    output_file.write_all(pop_file.as_bytes())?;
    Ok(())
}
