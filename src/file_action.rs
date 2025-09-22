use crate::bar::ProgressBar;
use crate::installer::Installer;
use crate::wildcards::*;
use crate::Args;

use std::fs;
use std::io::{self, stdout, Read, Write};

use flate2::read::GzDecoder;
use tar::Archive;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum FileAction {
    Extract {
        file: String,
        include: Box<[String]>,
        exclude: Option<Box<[String]>>,
        keep_folders: Option<bool>,

        #[serde(default, skip_serializing)]
        file_type: FileType,
    },
    SingleFile {
        file: String,
    },
    None,
}

#[derive(Debug, Deserialize, Default)]
pub enum FileType {
    Zip,
    Tar,
    TarGz,
    TarXz,
    #[default]
    Unsupported,
}

impl FileAction {
    pub fn validate(&mut self, tag: Option<&str>, name: &str) -> Result<(), String> {
        match self {
            FileAction::Extract {
                file,
                include,
                exclude,
                file_type,
                ..
            } => {
                if include.is_empty() {
                    return Err(format!("{name}: The include field must not be empty"));
                }
                if let Some(tag) = tag {
                    *include = include.iter().map(|p| p.replace("$tag", tag)).collect();
                    *exclude = exclude.as_ref().map_or_else(
                        || None,
                        |p| Some(p.iter().map(|p| p.replace("$tag", tag)).collect()),
                    );
                } else if include.concat().contains("$tag")
                    || exclude
                        .as_ref()
                        .is_some_and(|exclude| exclude.concat().contains("$tag"))
                {
                    return Err(format!("{name}: Use of missing field: `$tag`"));
                }

                Self::validate_file(file, tag, name)?;
                *file_type = Self::get_file_type(file);
                if matches!(file_type, FileType::Unsupported) {
                    return Err(format!("{name}: Unsupported file extension: `{file}`"));
                }
                Ok(())
            }
            FileAction::SingleFile { file } => {
                Self::validate_file(file, tag, name)?;
                Ok(())
            }
            FileAction::None => Err(format!("{name}: Action cannot be `None`")),
        }
    }

    pub fn validate_file(file: &mut String, tag: Option<&str>, name: &str) -> Result<(), String> {
        if !match_wildcard(file, "*.*") {
            return Err(format!(
                "{name}: File must specify an extension: \"{file}\"",
            ));
        }
        if file.ends_with('*') {
            return Err(format!("{name}: File must not end with a '*': \"{file}\"",));
        }
        if file.len() < 2 {
            return Err(format!("{name}: Invalid file: \"{file}\"",));
        }
        if let Some(tag) = tag {
            *file = file.replace("$tag", tag);
        } else if file.contains("$tag") {
            return Err(format!("{name}: Use of missing field: `$tag`"));
        }
        Ok(())
    }

    pub fn get_file_type(file: &str) -> FileType {
        let mut ext = file.split('.');
        match ext.next_back().unwrap() {
            "zip" => return FileType::Zip,
            "tar" => {
                println!(
                    "{}: `tar` support has not yet been tested",
                    orange!("WARNING")
                );
                return FileType::Tar;
            }
            "gz" => {
                if ext.next_back().unwrap() == "tar" {
                    return FileType::TarGz;
                }
            }
            "xz" => {
                // Currently disabled
                // if *ext.iter().next_back().unwrap() == "tar" {
                //     return FileType::TarXz;
                // }
            }
            _ => (),
        }
        FileType::Unsupported
    }

    pub fn stage_install(
        &mut self,
        installer: &mut Installer,
        data: Vec<u8>,
        extract_to: String,
        args: &Args,
    ) -> Result<(), String> {
        match self {
            FileAction::Extract {
                file,
                include,
                exclude,
                keep_folders,
                file_type,
            } => {
                let reader = std::io::Cursor::new(data);
                match file_type {
                    FileType::Zip => {
                        installer.files = Self::extract_zip(
                            args,
                            reader,
                            &extract_to,
                            include,
                            &exclude.take().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    FileType::Tar => {
                        installer.files = Self::extract_tar(
                            args,
                            Archive::new(reader),
                            &extract_to,
                            include,
                            &exclude.take().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?
                    }
                    FileType::TarGz => {
                        installer.files = Self::extract_tar_gz(
                            args,
                            reader,
                            &extract_to,
                            include,
                            &exclude.take().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    FileType::TarXz => {
                        installer.files = Self::extract_tar_xz(
                            args,
                            reader,
                            &extract_to,
                            include,
                            &exclude.take().unwrap_or_else(|| [].into()),
                            keep_folders.unwrap_or_default(),
                        )?;
                    }
                    FileType::Unsupported => {
                        return Err(format!("Unsupported archive extension: {file}"))
                    }
                }
            }
            FileAction::SingleFile { .. } => {
                let verbose = args.options.verbose || args.config.verbose_files;
                fs::create_dir_all(&extract_to).map_err(|e| e.to_string())?;
                let file = installer
                    .source
                    .ref_direct_url()?
                    .split('/')
                    .next_back()
                    .unwrap();

                let mut progress_bar = ProgressBar::new("Staging:");

                match verbose {
                    true => {
                        println!("Staging:");
                        print!("   {file} ... ");
                        let _ = stdout().flush();
                    }
                    false => progress_bar.update_progress(0.0, " 0 / 1"),
                }

                installer.files.push(file.to_string());
                fs::write(extract_to + file, data).map_err(|e| {
                    progress_bar.fail();
                    println_red!("{e}");
                    e.to_string()
                })?;

                match verbose {
                    true => println_green!("Done"),
                    false => {
                        progress_bar.update_progress(1.0, " 1 / 1");
                        progress_bar.pass();
                    }
                }
            }
            FileAction::None => panic!(),
        }

        Ok(())
    }

    fn extract_zip(
        args: &Args,
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let verbose = args.options.verbose || args.config.verbose_files;
        match verbose {
            true => println!("Staging:"),
            false => print!("… Staging…"),
        }

        let mut progress_bar = ProgressBar::new("Staging:");

        let mut zip_archive = zip::ZipArchive::new(reader).map_err(|e| {
            if !verbose {
                progress_bar.fail();
            }
            println_red!("Failed to read the archive");
            e.to_string()
        })?;

        let mut file_count = 0.0;
        let mut files: Vec<String> = zip_archive
            .file_names()
            .map(ToString::to_string)
            .filter(|file| {
                if file.ends_with('/') {
                    if keep_folders {
                        // NOTE: This creates all paths, regardless if they're included or not
                        let _ = fs::create_dir_all(extract_to.to_owned() + file).inspect_err(|e| {
                            println!("   Directory creation error: {}", format_red!("{e}"));
                        });
                    }
                    return false;
                }
                if match_any_wildcard(file, include) && !match_any_wildcard(file, exclude) {
                    file_count += 1.0;
                    true
                } else {
                    false
                }
            })
            .collect();

        fs::create_dir_all(extract_to).map_err(|e| {
            if !verbose {
                progress_bar.fail();
            }
            println!("Directory creation error: {}", format_red!("{e}"));
            e.to_string()
        })?;

        let mut files_processed = 0.0;

        for file in &mut files {
            files_processed += 1.0;

            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = stdout().flush();
                }
                false => progress_bar.update_progress(
                    files_processed / file_count,
                    &format!(" {files_processed} / {file_count}"),
                ),
            }

            let mut file_contents = Vec::new();
            zip_archive
                .by_name(file)
                .map_err(|e| {
                    match verbose {
                        true => println_red!("{e}"),
                        false => progress_bar.fail(),
                    }
                    e.to_string()
                })?
                .read_to_end(&mut file_contents)
                .map_err(|e| {
                    match verbose {
                        true => println_red!("{e}"),
                        false => progress_bar.fail(),
                    }
                    e.to_string()
                })?;

            if !keep_folders {
                *file = file.split('/').next_back().unwrap().to_owned();
            }

            fs::write(extract_to.to_owned() + file, file_contents).map_err(|e| {
                match verbose {
                    true => println_red!("{e}"),
                    false => progress_bar.fail(),
                }
                e.to_string()
            })?;

            if verbose {
                println_green!("Done");
            }
        }

        if !verbose {
            progress_bar.pass();
        }

        Ok(files)
    }

    fn extract_tar<R: io::Read>(
        args: &Args,
        mut archive: Archive<R>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let verbose = args.options.verbose || args.config.verbose_files;
        match verbose {
            true => println!("Staging:"),
            false => {
                print!("… Staging…");
                let _ = stdout().flush();
            }
        }

        let mut progress_bar = ProgressBar::new("Staging:");

        fs::create_dir_all(extract_to).map_err(|e| {
            if !verbose {
                progress_bar.fail();
            }
            println!("Directory creation error: {}", format_red!("{e}"));
            e.to_string()
        })?;

        let mut files_processed = 0.0;
        let mut fonts = Vec::new();
        let entries = archive.entries().map_err(|e| e.to_string())?;
        for mut entry in entries {
            let entry = entry.as_mut().unwrap();
            let file = match keep_folders {
                true => entry.path().unwrap().to_str().unwrap().to_owned(),
                false => entry
                    .path()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split('/')
                    .next_back()
                    .unwrap()
                    .to_string(),
            };

            if !match_any_wildcard(&file, include) || match_any_wildcard(&file, exclude) {
                continue;
            }

            match verbose {
                true => {
                    print!("   {file} ... ");
                    let _ = stdout().flush();
                }
                false => {
                    files_processed += 1.0;
                    progress_bar
                        .update_progress(1.0, &format!(" {files_processed} / {files_processed}"));
                }
            }

            if file.is_empty() || file.ends_with('/') {
                if keep_folders {
                    // NOTE: This creates all paths, regardless if they're included or not
                    fs::create_dir_all(extract_to.to_owned() + &file).map_err(|e| {
                        if verbose {
                            println_red!("{e}");
                        } else {
                            progress_bar.fail();
                            println!("{file}: {}", format_red!("{e}"));
                        }
                        e.to_string()
                    })?;
                }
                continue;
            }

            let mut file_contents = Vec::new();
            entry.read_to_end(&mut file_contents).map_err(|e| {
                if verbose {
                    println_red!("{e}");
                } else {
                    progress_bar.fail();
                    println!("{file}: {}", format_red!("{e}"));
                }
                e.to_string()
            })?;

            fs::write(&(extract_to.to_owned() + &file), file_contents).map_err(|e| {
                if !verbose {
                    progress_bar.fail();
                    println!("{file}: {}", format_red!("{e}"));
                }
                e.to_string()
            })?;
            fonts.push(file);

            if verbose {
                println_green!("Done");
            }
        }

        if !verbose {
            progress_bar.pass();
        }

        Ok(fonts)
    }

    fn extract_tar_gz(
        args: &Args,
        reader: std::io::Cursor<Vec<u8>>,
        extract_to: &str,
        include: &[String],
        exclude: &[String],
        keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        let _ = stdout().flush();

        let mut tar_gz_archive = GzDecoder::new(reader);

        Self::extract_tar(
            args,
            Archive::new(&mut tar_gz_archive),
            extract_to,
            include,
            exclude,
            keep_folders,
        )
    }

    fn extract_tar_xz(
        _args: &Args,
        _reader: std::io::Cursor<Vec<u8>>,
        _extract_to: &str,
        _include: &[String],
        _exclude: &[String],
        _keep_folders: bool,
    ) -> Result<Vec<String>, String> {
        todo!("XZ format is currently unsupported");

        // NOTE: `tar.xz` support is currently disabled due to
        // outdated `xz` crate dependencies for `zip` and `bzip2`

        // let _ = stdout().flush();

        // let mut tar_xz_archive = XzDecoder::new(reader);

        // Self::extract_tar(
        //     args,
        //     Archive::new(&mut tar_gz_archive),
        //     extract_to,
        //     include,
        //     exclude,
        //     keep_folders,
        // )
    }

    pub fn ref_file(&self) -> Result<&str, String> {
        match self {
            FileAction::Extract { file, .. } | FileAction::SingleFile { file } => Ok(file),
            FileAction::None => Err(format!(
                "Cannot obtain field `file` from `{:?}`",
                FileAction::None
            )),
        }
    }

    pub const fn take(&mut self) -> Self {
        let mut output = Self::None;
        std::mem::swap(self, &mut output);
        output
    }
}
