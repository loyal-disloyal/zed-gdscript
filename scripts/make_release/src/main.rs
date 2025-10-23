/// This script prompts if you want to create a major, minor, or patch release
/// of the repository and automatically pushes changes to the Zed extensions
/// directory, assuming it's located at a specific path.
///
/// It changes the version numbers in this repository and then it changes the
/// extension version number in the extensions repository and updates the
/// submodule.
use dialoguer::Select;
use semver::Version;
use std::fs;
use std::process::Command;
use toml::{Table, Value};

const ZED_GDSCRIPT_REPO_DIR: &str = "../..";
const ZED_EXTENSIONS_DIR: &str = "../../../../third-party/zed-extensions";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !std::path::Path::new(ZED_EXTENSIONS_DIR).exists() {
        eprintln!(
            "Error: The zed extensions directory does not exist at {}.",
            ZED_EXTENSIONS_DIR
        );
        std::process::exit(1);
    }
    let output = Command::new("git").args(["-C", ZED_EXTENSIONS_DIR, "status", "--porcelain"]).output()?;
    if !output.stdout.is_empty() {
        eprintln!("Error: The Zed extensions git working directory is not clean.");
        std::process::exit(1);
    }

    std::env::set_current_dir(ZED_GDSCRIPT_REPO_DIR)?;
    if !std::path::Path::new("extension.toml").exists() {
        eprintln!("Error: extension.toml not found in the zed-gdscript repository.");
        std::process::exit(1);
    }
    let output = Command::new("git").args(["status", "--porcelain"]).output()?;
    if !output.stdout.is_empty() {
        eprintln!("Error: The zed-gdscript git working directory is not clean. Please commit or stash changes before running this script.");
        std::process::exit(1);
    }

    let cargo_toml: Table = toml::from_str(&fs::read_to_string("Cargo.toml")?)?;
    let current_version_str = cargo_toml["package"]["version"].as_str().unwrap();
    let current_version = Version::parse(current_version_str)?;

    let options = vec!["minor", "major", "patch"];
    let selection = Select::new()
        .with_prompt("Select release type:")
        .items(&options)
        .default(0)
        .interact()?;
    let bump_type = options[selection];

    let new_version = match bump_type {
        "major" => Version::new(current_version.major + 1, 0, 0),
        "minor" => Version::new(current_version.major, current_version.minor + 1, 0),
        "patch" => Version::new(
            current_version.major,
            current_version.minor,
            current_version.patch + 1,
        ),
        _ => unreachable!(),
    };
    let new_version_str = new_version.to_string();

    let mut cargo_toml = cargo_toml;
    cargo_toml["package"]["version"] = Value::String(new_version_str.clone());
    fs::write("Cargo.toml", toml::to_string(&cargo_toml)?)?;
    println!("Updated Cargo.toml to version {}", new_version_str);

    let mut extension_toml: Table = toml::from_str(&fs::read_to_string("extension.toml")?)?;
    extension_toml["version"] = Value::String(new_version_str.clone());
    fs::write("extension.toml", toml::to_string(&extension_toml)?)?;
    println!("Updated extension.toml to version {}", new_version_str);

    Command::new("git")
        .args(["add", "Cargo.toml", "extension.toml"])
        .status()?;
    Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("Bump version to {}", new_version_str),
        ])
        .status()?;

    Command::new("git")
        .args(["tag", &new_version_str])
        .status()?;
    println!("Created git tag: {}", new_version_str);

    Command::new("git").args(["push"]).status()?;
    Command::new("git").args(["push", "--tags"]).status()?;

    std::env::set_current_dir(ZED_EXTENSIONS_DIR)?;

    let branch_name = format!("update-gdscript-v{}", new_version_str);
    Command::new("git")
        .args(["checkout", "-b", &branch_name])
        .status()?;

    let mut zed_extensions_toml: Table = toml::from_str(&fs::read_to_string("extension.toml")?)?;
    zed_extensions_toml["gdscript"]["version"] = Value::String(new_version_str.clone());
    fs::write("extension.toml", toml::to_string(&zed_extensions_toml)?)?;

    Command::new("git")
        .args(["submodule", "update", "--remote", "extensions/gdscript"])
        .status()?;

    Command::new("git")
        .args(["add", "extension.toml"])
        .status()?;
    Command::new("git")
        .args(["add", "extensions/gdscript"])
        .status()?;
    Command::new("git")
        .args([
            "commit",
            "-m",
            &format!("Update gdscript extension to version {}", new_version_str),
        ])
        .status()?;

    Command::new("git")
        .args(["push", "-u", "origin", &branch_name])
        .status()?;

    println!("Release completed successfully.");

    Ok(())
}
