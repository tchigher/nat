extern crate pretty_bytes;
extern crate libc;

use chrono::prelude::*;
use filetime::FileTime;
use ansi_term::Style;
use pretty_bytes::converter::convert;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::{fs, io};
use structopt::StructOpt;
use termion::color;
use users::{get_group_by_gid, get_user_by_uid, uid_t};
use libc::{S_IRGRP, S_IROTH, S_IRUSR, S_IWGRP, S_IWOTH, S_IWUSR, S_IXGRP, S_IXOTH, S_IXUSR};

#[derive(StructOpt, Debug)]
#[structopt(name = "nat", about = "the ls replacement you never knew you needed")]
pub struct Cli {
  #[structopt(parse(from_os_str), default_value = ".", help = "Give me a directory")]
  path: std::path::PathBuf,

  #[structopt(
    default_value = "",
    short = "f",
    long = "file",
    help = "File to search for"
  )]
    file: String,

    #[structopt(short = "l", long = "headline", help = "Enables helper headline")]
    headline_on: bool,

    #[structopt(short = "a", long = "arehidden", help = "Shows hidden files")]
    hidden_files: bool,

    #[structopt(short = "w", long = "wide", help = "Enables wide mode output")]
    wide_mode: bool,

    #[structopt(short = "t", long = "time", help = "Disables the file time modified output")]
    time_on: bool,
    
    #[structopt(short = "s", long = "size", help = "Disables file size output")]
    size_on: bool,

    #[structopt(short = "g", long = "group", help = "Disables the file group output")]
    group_on: bool,

    #[structopt(short = "p", long = "perms", help = "Disables the permissions output")]
    perms_on: bool,

    #[structopt(short = "u", long = "user", help = "Disables the file user output")]
    user_on: bool,

    #[structopt(long = "nsort", help = "Turns off sorting")]
    is_sorted: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();
    let directory = &args.path;
    let headline_on = &args.headline_on;
    let hidden_files = &args.hidden_files;
    let wide_mode = &args.wide_mode;
    let time_on = &args.time_on;
    let size_on = &args.size_on;
    let group_on = &args.group_on;
    let perms_on = &args.perms_on;
    let user_on = &args.user_on;
    let is_sorted = &args.is_sorted;

    let entries = fs::read_dir(directory)?
      .map(|res| res.map(|e| e.path()))
      .collect::<Result<Vec<_>, io::Error>>()?;

    let mut size_count = 4;
    let mut group_size = 8;
    for s in &entries {
      if convert(fs::symlink_metadata(&s)?.size() as f64).len() > size_count {
        size_count = convert(fs::symlink_metadata(&s)?.size() as f64).len();
      };

      let metadata_uid = fs::symlink_metadata(&s)?.uid();
      let user_name_len = get_user_name(metadata_uid).len();
      if user_name_len > group_size {
        group_size = user_name_len;
      }

    }

    let mut found = false;

    draw_headlines(*headline_on);


    if &args.file != "" {
      for e in &entries {
        if e
          .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_lowercase()
            .contains(&args.file.to_lowercase())
        {
          let _ = single(e, size_count, *wide_mode);
          found = true;
        }
      }
      if !found {
        print!("{}", color::Fg(color::Red));
        println!(
          "{}",
          Style::new()
          .bold()
          .paint(format!("{} could not be found", &args.file))
        );
      }
      std::process::exit(0);
    }

    let mut dirs: Vec<&std::path::PathBuf> = vec![];

    for e in &entries {
      if !&e.file_name().unwrap().to_str().unwrap().starts_with(".") ||*hidden_files {

        if *&e.is_file() && !*is_sorted {
          dirs.push(*&e);
        } else {

        if !perms_on {
          let _ = file_perms(&e);
        }

        if !size_on {
          let _ = file_size(size_count, &e);
        }

        if !time_on {
          let _ = time_mod(e);
        }

        if !group_on {
          let _ = show_group_name(e);
        }

        if !user_on {
          let _ = show_user_name(e);
        }

        let _ = show_file_name(&e, *wide_mode);
        }
      }
    }
    for e in &dirs {
      let _ = single(e, size_count, *wide_mode);
    }

    Ok(())
}


pub fn draw_headlines(on: bool) {
  if on {
    draw_headline("permissions", 0, false);
    draw_headline("size", 0, true);
    draw_headline("last modified", 0, true);
    draw_headline("user", 0, true);
    draw_headline("name", 0, true);
    print!("\n");
  } 
}

pub fn file_perms(e: &&std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  let mode = fs::symlink_metadata(&e)?.permissions().mode();
  print!("{}", color::Fg(color::White));
  print!("{} ", perms(mode as u16));
  Ok(())
}

pub fn file_size(size_count: usize, e: &&std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  for _ in 0..(size_count - convert(fs::symlink_metadata(&e)?.size() as f64).len()) {
    print!(" ");
  }
  print!("{}", color::Fg(color::Green));
  print!(
    "{} ",
    Style::new()
    .bold()
    .paint(convert(fs::symlink_metadata(&e)?.size() as f64))
  );
  Ok(())
}

pub fn time_mod(e: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  if let Ok(_) = e.symlink_metadata()?.modified() {
    let mtime = FileTime::from_last_modification_time(&e.symlink_metadata().unwrap());
    let d = NaiveDateTime::from_timestamp(mtime.seconds() as i64, 0);
    print!("{}", color::Fg(color::LightRed));
    let datetime =  d;
    print!("{} ", datetime.format("%b %e %T"));
  }
  Ok(())
}

pub fn show_group_name(e: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  print!("{}", color::Fg(color::LightBlue));

  print!(
    "{} ",
    Style::new().bold().paint(
      get_group_by_gid(fs::symlink_metadata(e)?.gid())
      .unwrap()
      .name()
      .to_str()
      .unwrap()
    )
  );
  Ok(())
}

pub fn show_user_name(e: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
  print!("{}", color::Fg(color::LightYellow));
  print!(
    "{} ",
    Style::new().bold().paint(
      get_user_by_uid(fs::symlink_metadata(e)?.uid())
      .unwrap()
      .name()
      .to_str()
      .unwrap()
    )
  );
  Ok(())
}

pub fn show_file_name(e: &&std::path::PathBuf, wide_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
  print!("{}", color::Fg(color::White));
  if e.symlink_metadata()?.is_dir() {
    print!("{}", color::Fg(color::LightBlue));
    print!("{}/", &e.file_name().unwrap().to_str().unwrap());
    if !wide_mode {
      print!("\n");
    } else {
      print!(" ");
    }
  } else {
    print!("{}", color::Fg(color::LightGreen));
    print!(
      "{}",
      Style::new()
      .bold()
      .paint(e.file_name().unwrap().to_str().unwrap())
    );
    if !wide_mode {
      print!("\n");
    } else {
      print!(" ");
    }
  }
  print!("{}", color::Fg(color::Reset));
  Ok(())
}

pub fn draw_headline(input: &str, line_length: usize, print_separator: bool) {
  if print_separator {
    print!(" {}", Style::new().underline().paint(input));
  } else {
    print!("{}", Style::new().underline().paint(input));
  }
  draw_line(line_length);
}

fn draw_line(to: usize) {
  for _ in 0..to {
    print!("{}", Style::new().underline().paint(" "));
  }
}

pub fn get_user_name(uid: uid_t) -> String {
  get_user_by_uid(uid)
    .unwrap()
    .name()
    .to_str()
    .unwrap()
    .to_string()
}


pub fn single(e: &std::path::PathBuf, size_count: usize, wide_mode: bool) -> Result<(), Box<dyn std::error::Error>> {
  let args = Cli::from_args();

  if !&args.perms_on {
    let _ = file_perms(&e);
  }
  
  if !&args.size_on {
    let _ = file_size(size_count, &e);
  }

  if !&args.time_on {
    let _ = time_mod(e);
  }

  if !&args.group_on {
    let _ = show_group_name(e);
  }

  if !&args.user_on {
    let _ = show_user_name(e);
  }
  let _ = show_file_name(&e, wide_mode);
  Ok(())
}

pub fn perms(mode: u16) -> String {
  let user = triplet(mode, S_IRUSR as u16, S_IWUSR as u16, S_IXUSR as u16);
  let group = triplet(mode, S_IRGRP as u16, S_IWGRP as u16, S_IXGRP as u16);
  let other = triplet(mode, S_IROTH as u16, S_IWOTH as u16, S_IXOTH as u16);
  [format!("{}{}", color::Fg(color::Blue), user), format!("{}{}", color::Fg(color::LightRed), group), format!("{}{}", color::Fg(color::Yellow), other)].join("")
}

pub fn triplet(mode: u16, read: u16, write: u16, execute: u16) -> String {
  match (mode & read, mode & write, mode & execute) {
    (0, 0, 0) => "---",
    (_, 0, 0) => "r--",
    (0, _, 0) => "-w-",
    (0, 0, _) => "--x",
    (_, 0, _) => "r-x",
    (_, _, 0) => "rw-",
    (0, _, _) => "-wx",
    (_, _, _) => "rwx",
  }.to_string()
}
