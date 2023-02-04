use std::{cmp::min, path::Path};
use std::fs::File;
use std::io::Write;

use crossterm::style::Stylize;
use reqwest::Client;
use indicatif::{ProgressBar, ProgressStyle};
use futures_util::StreamExt;

pub fn download_file(client: &Client, url: &str, path: &Path) -> Result<(), String> {
	futures_lite::future::block_on(download_file_async(client, url, path))
}

pub async fn download_file_async(client: &Client, url: &str, path: &Path) -> Result<(), String> {
	// Reqwest setup
	let res = client
		.get(url)
		.send()
		.await
		.or(Err(format!("Failed to GET from '{}'", &url)))?;
	let total_size = res
		.content_length()
		.ok_or(format!("Failed to get content length from '{}'", &url))?;
	
	// Indicatif setup
	let pb = ProgressBar::new(total_size);
	pb.set_style(ProgressStyle::default_bar()
		.template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})").unwrap()
		.progress_chars("#>-"));
	pb.set_message(format!("Downloading {}", url));

	// download chunks
	let mut file = File::create(path).or(Err(format!("Failed to create file '{:?}'", path)))?;
	let mut downloaded: u64 = 0;
	let mut stream = res.bytes_stream();

	while let Some(item) = stream.next().await {
		let chunk = item.or(Err(format!("Error while downloading file")))?;
		file.write_all(&chunk)
			.or(Err(format!("Error while writing to file")))?;
		let new = min(downloaded + (chunk.len() as u64), total_size);
		downloaded = new;
		pb.set_position(new);
	}

	pb.finish_with_message(format!("Downloaded {} to {:?}", url, path));
	return Ok(());
}

pub fn unzip(archive: &Path, dest: &Path) -> Result<(), String> {
	println!("{}", "Extracting...".cyan());

	let file = std::fs::File::open(archive)
		.or(Err(String::from("Unable to open archive file")))?;

	let mut archive = zip::ZipArchive::new(file)
		.or(Err(String::from("Unable to open archive")))?;

	for i in 0..archive.len() {
		let mut file = archive.by_index(i).unwrap();
		let outpath = match file.enclosed_name() {
			Some(path) => dest.join(
				match path.to_owned().strip_prefix("MultiMC") {
					Ok(t) => t,
					Err(_) => continue
				}
			),
			None => continue,
		};

		if (*file.name()).ends_with('/') {
			std::fs::create_dir_all(&outpath)
				.or(Err(format!("Unable to create directory {:?}", outpath)))?;
		} else {
			if let Some(p) = outpath.parent() {
				if !p.exists() {
					std::fs::create_dir_all(p)
						.or(Err(format!("Unable to create directory {:?}", p)))?;
				}
			}
			let mut outfile = std::fs::File::create(&outpath)
				.or(Err(format!("Unable to create file {:?}", outpath)))?;
			std::io::copy(&mut file, &mut outfile).unwrap();
		}

	}

	Ok(())

}
