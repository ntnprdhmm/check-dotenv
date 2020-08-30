use serde::{Deserialize, Serialize};

use colored::*;
use dialoguer::Input;
use glob::glob;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct Workspaces {
    packages: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct PackageJson {
    workspaces: Workspaces,
}

fn read_project_path() -> PathBuf {
    let path = match env::current_dir() {
        Ok(b) => b,
        Err(err) => panic!(err),
    };
    path
}

fn read_workspaces_paths() -> Vec<String> {
    let project_path = read_project_path();

    let package_json_path = project_path.join("package.json");
    let package_json_path_str = package_json_path.into_os_string().into_string().unwrap();

    let mut package_json_file = File::open(package_json_path_str).unwrap();
    let mut package_json_content = String::new();
    package_json_file
        .read_to_string(&mut package_json_content)
        .unwrap();
    let parsed_package_json: PackageJson = serde_json::from_str(&package_json_content).unwrap();
    let workspaces = parsed_package_json.workspaces.packages;

    workspaces
        .into_iter()
        .map(|workspace| {
            project_path
                .join(workspace)
                .into_os_string()
                .into_string()
                .unwrap()
        })
        .collect()
}

fn get_hashmap_from_env_file(file: File) -> HashMap<String, String> {
    let file_buf_reader = BufReader::new(file);

    let mut hashmap = HashMap::new();

    for line in file_buf_reader.lines() {
        let l = line.unwrap();
        let parts: Vec<&str> = l.split("=").collect();
        let key = format!("{}", parts[0]);
        let value = if parts.len() == 2 {
            format!("{}", parts[1])
        } else {
            String::new()
        };
        hashmap.insert(key, value);
    }

    hashmap
}

fn ask_create_from_example(dotenv_example_path: &String, dotenv_path: &String) {
    let input: String = Input::new()
        .with_prompt(format!(
            "Do you want to create a {} from {} ? (y/n)",
            ".env".cyan(),
            ".env.example".cyan()
        ))
        .validate_with(|input: &str| -> Result<(), &str> {
            if input == "y" || input == "n" {
                Ok(())
            } else {
                Err("please answer with 'y' or 'n'")
            }
        })
        .interact()
        .unwrap();

    if input == "y" {
        fs::copy(dotenv_example_path, dotenv_path).unwrap();
        println!("{} created !", ".env".cyan());
    }
}

fn print_missing_lines(missing_lines: &HashMap<&String, &String>) {
    if missing_lines.len() == 0 {
        return;
    }
    println!("{}", "These lines are missing".bold());
    for (key, value) in missing_lines.iter() {
        let line = format!("{}={}", key, value);
        println!("{}", line.green());
    }
}

fn print_useless_lines(useless_lines: &HashMap<&String, &String>) {
    if useless_lines.len() == 0 {
        return;
    }
    println!("{}", "These lines are useless".bold());
    for (key, value) in useless_lines.iter() {
        let line = format!("{}={}", key, value);
        println!("{}", line.red());
    }
}

fn ask_what_to_do(
    missing_lines: &HashMap<&String, &String>,
    useless_lines: &HashMap<&String, &String>,
    dotenv_example_path: &String,
    dotenv_path: &String,
) {
    print_missing_lines(&missing_lines);
    print_useless_lines(&useless_lines);

    if missing_lines.len() > 0 || useless_lines.len() > 0 {
        println!("{}", "What do you want to do ? (r/n)".white().bold());
        println!("- replace {} (r)", ".env".cyan());
        println!("- do nothing (n)");

        let input: String = Input::new()
            .validate_with(|input: &str| -> Result<(), &str> {
                if input == "r" || input == "n" {
                    Ok(())
                } else {
                    Err("please answer with 'r' or 'n'")
                }
            })
            .interact()
            .unwrap();

        if input == "r" {
            fs::copy(dotenv_example_path, dotenv_path).unwrap();
            println!("{} updated !", ".env".cyan());
        }
    }
}

fn compare_package_dotenv(path: &str) -> Result<(), ()> {
    println!("{}", path.yellow().bold());

    let dotenv_example_path = Path::new(path).join(".env.example");
    let dotenv_path = Path::new(path).join(".env");

    let dotenv_example_path_str = dotenv_example_path.into_os_string().into_string().unwrap();
    let dotenv_path_str = dotenv_path.into_os_string().into_string().unwrap();

    // read dot example first. If not exists -> exit
    let dotenv_example = match File::open(&dotenv_example_path_str) {
        Ok(f) => f,
        Err(_e) => {
            println!("Nothing to do here");
            return Ok(());
        }
    };

    // read .env
    let dotenv = match File::open(&dotenv_path_str) {
        Err(_e) => {
            // if does not exists, ask create and exit
            println!("{} not found", ".env".cyan());
            ask_create_from_example(&dotenv_example_path_str, &dotenv_path_str);
            return Ok(());
        }
        Ok(f) => f,
    };

    // read env files in hashmap
    let dotenv_hashmap = get_hashmap_from_env_file(dotenv);
    let dotenv_example_hashmap = get_hashmap_from_env_file(dotenv_example);

    // find missing lines in .env

    let mut missing_lines = HashMap::new();

    for (key, value) in dotenv_example_hashmap.iter() {
        if !dotenv_hashmap.contains_key(key) {
            missing_lines.insert(key, value);
        }
    }

    // find useless lines in .env.example

    let mut useless_lines = HashMap::new();

    for (key, value) in dotenv_hashmap.iter() {
        if !dotenv_example_hashmap.contains_key(key) {
            useless_lines.insert(key, value);
        }
    }

    // show diff and ask what to do

    ask_what_to_do(
        &missing_lines,
        &useless_lines,
        &dotenv_example_path_str,
        &dotenv_path_str,
    );

    Ok(())
}

fn main() {
    let workspaces_paths = read_workspaces_paths();
    println!("Workspaces: {:?}", workspaces_paths);

    let paths = glob(&workspaces_paths[0]).unwrap();

    for path_result in paths {
        let raw_path = path_result.unwrap();
        let path = raw_path.to_str().unwrap();
        compare_package_dotenv(path).unwrap();
    }
}
