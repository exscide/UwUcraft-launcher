use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::env::current_exe;

use git2::Repository;
use reqwest::Client;
use walkdir::WalkDir;
use crossterm::style::Stylize;


mod pull;
mod download;

const REPO_URL: &'static str = "https://github.com/exscide/UwUcraft";


fn update(path: &Path) -> Result<(), String> {
	// fetch / update repo

	let repo = match Repository::open(path) {
		Ok(repo) => repo,
		Err(_) => {

			let new_repo = Repository::init(path)
				.or(Err(String::from("Unable to initialize repository")))?;

			new_repo.remote("origin", REPO_URL)
				.or(Err(String::from("Unable to add remote")))?;

			new_repo
		}
	};

	println!("{}", "Updating...".cyan());

	let mut remote = repo.find_remote("origin")
		.or(Err(String::from("Unable to obtain remote")))?;

	let fetch_commit = pull::do_fetch(&repo, &["master"], &mut remote)
		.or(Err(String::from("Unable to fetch commit")))?;
	pull::do_merge(&repo, "master", fetch_commit)
		.or(Err(String::from("Unable to merge")))?;

	Ok(())
}

fn download_launcher(dest: &Path) -> Result<(), String> {
	let mut path = PathBuf::from(".tmp");
	std::fs::create_dir_all(&path)
		.or(Err(String::from("Unable to create .tmp directory")))?;
	path.push("mmc-stable-windows.zip");
	download::download_file(&Client::new(), "https://files.multimc.org/downloads/mmc-stable-windows.zip", &path)?;

	download::unzip(&path, dest)
}

fn launch_game(base: PathBuf) -> Result<(), String> {
	// cd base
	std::env::set_current_dir(&base)
		.or(Err(String::from("Unable to change directory")))?;

	let mut launcher_path = base.clone();
	launcher_path.push("launcher");

	std::fs::create_dir_all(&launcher_path)
		.or(Err(String::from("Unable to create launcher directory")))?;

	if !launcher_path.join("MultiMC.exe").is_file() {
		download_launcher(&launcher_path)?;

		let mut fbat = std::fs::File::create(launcher_path.join("multimc.cfg"))
			.or(Err(String::from("Unable to create multimc.cfg")))?;
		fbat.write_all(include_bytes!("multimc.cfg"))
			.or(Err(String::from("Unable to write multimc.cfg")))?;
	}

	let lbat = base.join("launch.bat");
	if !lbat.is_file() {
		let mut fbat = std::fs::File::create(lbat)
			.or(Err(String::from("Unable to create launch.bat")))?;
		fbat.write_all(include_bytes!("launch.bat"))
			.or(Err(String::from("Unable to write launch.bat")))?;
	}

	println!("{}", "Launching, be patient...".cyan());

	// dirty hack to ignore output of MultiMC
	Command::new("cmd")
		.args(["/c", "launch.bat"])
		.stdin(Stdio::null())
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.spawn()
		.or(Err(String::from("Failed to start launcher")))?;

	// Command::new(launcher)
	// 	.args(["-l", "UwUcraft"])
	// 	.stdin(Stdio::null())
	// 	.stdout(Stdio::null())
	// 	.stderr(Stdio::null())
	// 	.spawn()
	// 	.or(Err(String::from("Failed to start launcher")))?;

	Ok(())
}

fn apply_patches(src: &Path, dst: &Path) -> Result<(), String> {
	let reg: Vec<PathBuf> = std::fs::read_to_string(src.join("overwrite.txt"))
			.or(Err(String::from("Unable read overwrite.txt")))?
			.lines()
			.filter_map(|f| {
				let f = f.trim();
				if f == "" {
					None
				} else {
					Some(PathBuf::from(f))
				}
			})
			.collect();

	// delete all files to be overwritten
	for item in &reg {
		let torm = dst.join(item);

		if torm.is_dir() {
			//println!("deleting {:?}", torm);
			std::fs::remove_dir_all(&torm)
				.or(Err(String::from(format!("Unable to delete {:?}", &torm))))?;
		} else if torm.is_file() {
			//println!("deleting {:?}", torm);
			std::fs::remove_file(&torm)
				.or(Err(String::from(format!("Unable to delete {:?}", &torm))))?;
		} else {
			//println!("missing  {:?}", torm);
		}
	}

	let mut copied = 0;

	for file in WalkDir::new(src).into_iter().filter_map(|file| file.ok()) {
		if file.path() == src { continue }

		let item = file.path().strip_prefix(src).unwrap();
		let dst_file = dst.join(item);

		if file.path().is_file() && !&dst_file.exists() {
			let mut base_dir = dst_file.clone();
			base_dir.pop();

			if !base_dir.is_dir() {
				//println!("creating directory {:?}", base_dir);
				std::fs::create_dir_all(base_dir).unwrap();
			}

			std::fs::copy(file.path(), &dst_file).unwrap();
			//println!("copied {:?}", &item);
			copied += 1;
		}
	}

	println!("{}{}{}{}", reg.len(), " files deleted, ".green(), copied, " files copied".green());

	Ok(())
}

#[tokio::main]
async fn main() {
	println!("{}", r###"
 __  __     __     __     __  __     ______     ______     ______     ______   ______  
/\ \/\ \   /\ \  _ \ \   /\ \/\ \   /\  ___\   /\  == \   /\  __ \   /\  ___\ /\__  _\ 
\ \ \_\ \  \ \ \/ ".\ \  \ \ \_\ \  \ \ \____  \ \  __<   \ \  __ \  \ \  __\ \/_/\ \/ 
 \ \_____\  \ \__/".~\_\  \ \_____\  \ \_____\  \ \_\ \_\  \ \_\ \_\  \ \_\      \ \_\ 
  \/_____/   \/_/   \/_/   \/_____/   \/_____/   \/_/ /_/   \/_/\/_/   \/_/       \/_/ 
																						
	"###.magenta());

	// base path
	let mut base = current_exe().expect("error code 0x0a");
	base.pop();
	// repo path
	let mut repo_path = base.clone();
	repo_path.push("_data");
	// instance path
	let mut instance_path = base.clone();
	instance_path.push("launcher/instances/UwUcraft");

	// update repo
	match update(&repo_path) {
		Ok(_) => {},
		Err(e) => {
			println!("{}", e.black().on_red());
			return;
		}
	}

	// apply patches from repo
	match apply_patches(&repo_path, &instance_path) {
		Ok(_) => {},
		Err(e) => {
			println!("{}", e.black().on_red());
			return;
		}
	}

	// launch the game
	match launch_game(base) {
		Ok(_) => {},
		Err(e) => {
			println!("{}", e.black().on_red());
			return;
		}
	}

	println!("\nPress Enter to exit");
	std::io::stdin().read_line(&mut String::new()).expect("error code 0x03");
}
