// Copyright (C) 2015 Sam Henson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#![feature(slice_patterns)]
#![feature(convert)]

use std::env;
use std::fs;
use std::io;

use std::io::Write;

use std::path::{PathBuf};
use std::process;

use tempdir::TempDir;

extern crate aws;
extern crate tempdir;

// ---------------------------------------------------------------------------------------------------------------------

fn do_upload (p : &Parameters, src_file : PathBuf) -> io::Result<()> {
    let tmp = try!(TempDir::new("log-upload"));

    let src_file2 = src_file.clone();
    let filename = src_file2.file_name().unwrap().to_str().unwrap();
    let src = src_file.to_str().unwrap();
    let enc = tmp.path().join(filename);

    // encrypt
    print!("gpg ");
    io::stdout().flush().unwrap();
    try!(process::Command::new("gpg").arg("--recipient").arg(&p.encrypt_key)
                                     .arg("--local-user").arg(&p.signing_key)
                                     .arg("--encrypt")
                                     .arg("--sign")
                                     .arg("-o").arg(&enc)
                                     .arg("--batch")
                                     .arg("--set-filename").arg("")
                                     .arg(src).status());

    // upload
    let s3_path : String = p.s3_path.clone() + filename;
    print!("s3 ");
    io::stdout().flush().unwrap();
    let result = aws::s3::put(&p.s3_bucket, &s3_path, &enc, &[]);
    if result.is_ok() {
        try!(fs::remove_file(&src_file));
    }
    result
}

// ---------------------------------------------------------------------------------------------------------------------

struct Parameters {
    s3_bucket     : String,
    s3_path       : String,
    log_dir       : PathBuf,
    encrypt_key   : String,
    signing_key   : String
}

fn do_main (p : &Parameters) -> io::Result<()> {
    for f in try!(fs::read_dir(&p.log_dir)) {
        let src_file = try!(f).path();

        print!("- {}: ", src_file.to_str().unwrap());
        try!(io::stdout().flush());

        match do_upload(p, src_file) {
            Ok(()) => {
                println!("ok ");
            },
            Err(e) => {
                println!("fail ");
                println!("{:?}", e);
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------

fn read_parameter (name : &str) -> Result<String, String> {
    match get_argv_string(name) {
        Ok( Some(x) ) => Ok(x),
        Ok( None    ) => Err(format!("Error: {} is required", name)),
        Err(x)        => Err(format!("Error: {:?}", x))
    }
}

fn read_parameters () -> Result<Parameters, String> {

    let s3_path_s = read_parameter("s3-path");
    let log_dir_s = read_parameter("log-dir");
    let encrypt_key_s = read_parameter("encrypt-key");
    let signing_key_s = read_parameter("signing-key");

    let s3_path_ok : String = try!(s3_path_s);
    let parts : Vec<&str> = s3_path_ok.splitn(2, '/').collect();
    let (s3_bucket, s3_path) : (String, String) = match parts.as_slice() {
        [bucket]       => (bucket.to_string(), "".to_string()),
        [bucket, path] => (bucket.to_string(), path.to_string()),
        _              => panic!("Error parsing s3-path")
    };

    Ok( Parameters {
        s3_bucket   : s3_bucket,
        s3_path     : s3_path,
        log_dir     : PathBuf::from(try!(log_dir_s)),
        encrypt_key : try!(encrypt_key_s),
        signing_key : try!(signing_key_s)
    } )
}

// ---------------------------------------------------------------------------------------------------------------------

fn get_argv_string (name : &str) -> io::Result<Option<String>> {
    let mut args = env::args();
    let arg_name = "--".to_string() + name;

    if args.find( |a| *a == arg_name ).is_some() {
        match args.next() {
            Some(a) =>
                if a.starts_with("--") {
                    Err( io::Error::new(io::ErrorKind::InvalidInput, format!("Expected a value after argument '{}'", arg_name)) )
                } else {
                    Ok(Some(a.clone()))
                },
            None    => Err( io::Error::new(io::ErrorKind::InvalidInput, format!("Expected a value after argument '{}'", arg_name)) ),
        }
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------------------------------------------------

fn print_stderr (msg : &str) {
    let mut stderr = io::stderr();
    io::copy(&mut String::from(msg).as_bytes(), &mut stderr).unwrap();
}

fn show_usage (msg : &str) {
    let mut args = env::args();
    print_stderr( &format!("Usage: {} --s3-path bucket/prefix --log-dir path --encrypt-key keyid --signing-key keyid\n{}\n", args.next().unwrap(), msg) );
}

// ---------------------------------------------------------------------------------------------------------------------

fn main () {
    match read_parameters() {
        Ok(p)  => {
            match do_main(&p) {
                Ok(_)  => { },
                Err(e) => print_stderr(&format!("{:?}", e))
            }
        },
        Err(s) => show_usage(&s)
    }
}

