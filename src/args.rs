// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

use std::path::PathBuf;

pub struct CliArgs {
  pub entry_point: PathBuf,
  pub out_dir: PathBuf,
  pub cjs: bool,
}

pub fn parse_cli_args() -> CliArgs {
  use clap::App;
  use clap::Arg;

  let matches = App::new("d2n")
    .version("0.1")
    .author("The Deno Authors")
    .about("Creates a Node distribution build of a Deno module.")
    .arg(
      Arg::with_name("entry-point")
        .help("File path to entry point module.")
        .index(1)
        .takes_value(true),
    )
    .arg(
      Arg::with_name("out")
        .long("out")
        .help("The output directory.")
        .takes_value(true),
    )
    .arg(
      // todo: better arg
      Arg::with_name("cjs")
        .long("cjs")
        .help("Output cjs instead of mjs files")
    )
    .get_matches();

  let entry_point =
    PathBuf::from(matches.value_of("entry-point").unwrap_or("mod.ts"))
      .canonicalize()
      .unwrap();
  let out_dir =
    PathBuf::from(matches.value_of("out").unwrap_or("dist"));
  let cjs = matches.is_present("cjs");
  CliArgs {
    entry_point,
    out_dir,
    cjs,
  }
}
