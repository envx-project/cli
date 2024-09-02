use std::path::Path;

use home::home_dir;

use super::*;

#[derive(Parser)]
pub struct Args {
    #[clap(short, long, default_value_t = false)]
    verbose: bool,
}

pub async fn command(args: Args) -> Result<()> {
    println!("Migrating config file... Please do not interrupt this process.");

    let mut old_config_path = home_dir().unwrap();
    old_config_path.push(".config/envcli/config.json");

    if !old_config_path.exists() {
        println!(
            "Config file not found at {}. Skipping migration.",
            old_config_path.to_str().unwrap()
        );
        return Ok(());
    }

    // mkdir at ~/.config/envx
    let mut new_config_path = home_dir().unwrap();
    new_config_path.push(".config/envx/config.json");
    if !new_config_path.parent().unwrap().exists() {
        std::fs::create_dir_all(new_config_path.clone()).unwrap();
    }

    if args.verbose {
        println!(
            "Copying {} to {}",
            old_config_path.to_str().unwrap(),
            new_config_path.to_str().unwrap()
        );
    }
    std::fs::copy(old_config_path.clone(), new_config_path)?;

    let mut old_key_dir = home_dir().unwrap();
    old_key_dir.push(".config/envcli/keys");
    let mut new_key_dir = home_dir().unwrap();
    new_key_dir.push(".config/envx/keys");

    if !old_key_dir.exists() {
        println!(
            "Key directory not found at {}. Skipping migration.",
            old_key_dir.to_str().unwrap()
        );
        return Ok(());
    }

    // mkdir at ~/.config/envx/keys
    if !new_key_dir.parent().unwrap().exists() {
        if args.verbose {
            println!(
                "Creating key directory at {}",
                new_key_dir.to_str().unwrap()
            );
        }
        std::fs::create_dir_all(new_key_dir.clone()).unwrap();
    }

    // copy all files from old key dir to new key dir
    for entry in std::fs::read_dir(old_key_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let old_path = entry.path();
            let new_dir_path = new_key_dir.join(old_path.file_name().unwrap());

            if !new_dir_path.exists() {
                if args.verbose {
                    println!(
                        "Creating key directory at {}",
                        new_dir_path.to_str().unwrap()
                    );
                }
                std::fs::create_dir_all(new_dir_path.clone()).unwrap();
            }

            for file in std::fs::read_dir(entry.path())? {
                let file = file?;
                if !file.file_type()?.is_file() {
                    continue;
                }
                let old_path = file.path();
                let new_path = new_dir_path.join(old_path.file_name().unwrap());
                if args.verbose {
                    println!(
                        "Copying {} to {}",
                        old_path.to_str().unwrap(),
                        new_path.to_str().unwrap()
                    );
                }

                if !old_path.to_str().unwrap().contains("private.key")
                    && !old_path.to_str().unwrap().contains("public.key")
                {
                    println!("{}", "Extraneous file found, copying anyway. Please make sure you have the correct files in your key directory.".red());
                    println!("File: {}", old_path.to_str().unwrap().red());
                }

                let result = std::fs::copy(old_path, new_path);

                match result {
                    Ok(_) => {}
                    Err(e) => {
                        println!("Failed to copy file: {}", e);
                    }
                }
            }
        }
    }

    std::fs::remove_dir_all(old_config_path)?;

    Ok(())
}
