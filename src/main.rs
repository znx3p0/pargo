use cargo_pargo::*;
use serde::Deserialize;
use std::{env, fs, path::PathBuf};
use toml::{
    value::{Map, Table},
    Value,
};

fn main() {
    env::set_var("RUST_LOG", "INFO");
    pretty_env_logger::init();
    let mut args = std::env::args().collect::<Vec<_>>();
    args.remove(0);

    // check environment if pargo should run or default to cargo
    Env::go_to_root();
    if Env::should_run() {
        let toml = Registry::new();
        if Env::is_not_init() {
            log::info!("initializing pargo");
            Env::init();
        }
        let pargo = toml.pargo.unwrap_or_else(|| {
            let mut map = Map::new();
            map.insert("path".into(), Value::String("pargo.rs".into()));
            map
        });
        let path = pargo["path"].as_str().unwrap();
        let mut should_compile = false;
        if Env::should_update_script(path) {
            log::info!("should update script");
            fs::copy(pargo["path"].as_str().unwrap(), ".pargo/pargo/src/main.rs").unwrap();
            should_compile = true;
        }
        if Env::should_update_toml() {
            log::info!("should update toml");
            let mut toml: Registry = toml::from_slice(&fs::read("Pargo.toml").unwrap()).unwrap();
            toml.dependencies = pathify(toml.dependencies);

            let mut p: Table =
                toml::from_slice(&fs::read(".pargo/pargo/Cargo.toml").unwrap()).unwrap();
            p["dependencies"] = Value::Table(toml.dependencies);
            let p = toml::to_string(&p).unwrap();
            fs::write(".pargo/pargo/Cargo.toml", p).unwrap();
            should_compile = true;
        }
        if should_compile {
            log::info!("compiling pargo script");
            env::set_current_dir(".pargo/pargo").unwrap();
            cargo!("build").run().unwrap();
            env::set_current_dir("..").unwrap();
            env::set_current_dir("..").unwrap();
        }
        log::info!("running pargo script");
        std::process::Command::new(".pargo/pargo/target/debug/pargo")
            .args(args)
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    } else {
        // run cargo
        Cargo::from_args().run().unwrap();
    }
}

fn pathify(mut m: Map<String, Value>) -> Map<String, Value> {
    for (_, val) in m.iter_mut() {
        if let Value::Table(map) = val {
            if map.contains_key("path") {
                let p = map["path"].clone();
                if let Value::String(s) = p {
                    map["path"] = Value::String(format!("../../{}", s));
                }
            }
        }
    }
    m
}

struct Env;
impl Env {
    fn should_update_script(path: &str) -> bool {
        let first = seahash::hash(&fs::read(path).unwrap());
        let second = seahash::hash(&fs::read(".pargo/pargo/src/main.rs").unwrap());
        first != second
    }
    fn should_update_toml() -> bool {
        let mut toml: Registry = toml::from_slice(&fs::read("Pargo.toml").unwrap()).unwrap();
        let pargo_toml: Registry =
            toml::from_slice(&fs::read(".pargo/pargo/Cargo.toml").unwrap()).unwrap();
        toml.dependencies = pathify(toml.dependencies);
        pargo_toml.dependencies != toml.dependencies
    }
    fn init() {
        fs::create_dir(".pargo").ok();
        env::set_current_dir(".pargo").unwrap();
        cargo!("init", "pargo").run().unwrap();
        env::set_current_dir("..").unwrap();
    }
    fn is_not_init() -> bool {
        fs::read_dir(".pargo/").is_err()
    }
    fn should_run() -> bool {
        fs::read("Pargo.toml").is_ok()
    }
    fn go_to_root() {
        let mut parent_dir = PathBuf::from(".");
        loop {
            let contains_rs = fs::read_dir(".")
                .unwrap()
                .into_iter()
                .filter(|s| s.is_ok())
                .map(|f| f.unwrap())
                .map(|s| {
                    let p = s.file_name();
                    p.to_string_lossy().ends_with(".rs")
                })
                .any(|t| t);

            match fs::read("Cargo.toml").is_ok() || contains_rs {
                true => {
                    parent_dir = std::env::current_dir().unwrap();
                    std::env::set_current_dir("..").unwrap()
                }
                false => {
                    std::env::set_current_dir(parent_dir).unwrap();
                    break;
                }
            };
        }
    }
}

#[derive(Deserialize, Debug)]
struct Registry {
    dependencies: Table,
    pargo: Option<Table>,
}

impl Registry {
    fn new() -> Self {
        let toml = fs::read("Pargo.toml").unwrap();
        toml::from_slice(&toml).unwrap()
    }
}
