//! This crate allows you to permanently set environment variables
//!
//! # Examples
//! ```rust
//! // Check if DUMMY is set, if not set it to 1
//! // export DUMMY=1
//! env_perm::check_or_set("DUMMY", 1).expect("Failed to find or set DUMMY");
//! // Append $HOME/some/cool/bin to $PATH
//! // export PATH= "$HOME/some/cool/bin:$PATH"
//! env_perm::append("PATH", "$HOME/some/cool/bin").expect("Couldn't find PATH");
//! // Sets a variable without checking if it exists.
//! // Note you need to use a raw string literal to include ""
//! // export DUMMY="/something"
//! env_perm::set("DUMMY", r#""/something""#).expect("Failed to set DUMMY");
//! ```

use std::env;
use std::fmt;

#[cfg(target_family = "windows")]
use std::process::Command;
#[cfg(target_family = "windows")]
use std::io;

#[cfg(target_family = "unix")]
use std::{fs::{File, OpenOptions},path::PathBuf};
#[cfg(target_family = "unix")]
use dirs;
#[cfg(target_family = "unix")]
use std::io::{self, Write};


/// Checks if a environment variable is set.
/// If it is then nothing will happen.
/// If it's not then it will be added
/// to your profile.
pub fn check_or_set<T, U>(var: T, value: U) -> io::Result<()>
where T: fmt::Display + AsRef<std::ffi::OsStr>,
      U: fmt::Display,
{
    env::var(&var)
        .map(|_|())
        .or_else(|_| set(var, value))
}


/// Appends a value to an environment variable
/// Useful for appending a value to PATH
#[cfg(target_family = "unix")]
pub fn append<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}=\"{}:${}\"", var, value, var)?;
    profile.flush()
}
#[cfg(target_family = "windows")]
pub fn append<T: fmt::Display>(var: T, value: T) -> io::Result<()> {
    let string_var = format!("{}",var);
    let current_value = env::var(string_var);
    match current_value {
        Ok(current_value) => {
            set(&var, format!("{}; {}",value, current_value))
        }
        Err(var_error) => {
            let reason = match var_error {
                env::VarError::NotPresent => {"Not present".to_owned()}
                env::VarError::NotUnicode(x) => {format!("Non unicode value {:?}", x)}
            };
            let message = format!("Could not environment variable {}. Reason: {}", var, &reason);
            Err(io::Error::new(io::ErrorKind::Other, message))
        }
    }
}

/// Sets an environment variable without checking
/// if it exists.
/// If it does you will end up with two
/// assignments in your profile.
/// It's recommended to use `check_or_set`
/// unless you are certain it doesn't exist.
#[cfg(target_family = "unix")]
pub fn set<T: fmt::Display, U: fmt::Display>(var: T, value: U) -> io::Result<()> {
    let mut profile = get_profile()?;
    writeln!(profile, "\nexport {}={}", var, value)?;
    profile.flush()
}
#[cfg(target_family = "windows")]
pub fn set<T: fmt::Display, U: fmt::Display>(var: T, value: U) -> io::Result<()> {
    let var = format!("{}", var);
    let value = format!("\"{}\"", value);
    let output =Command::new("setx").arg(var).arg(value).output();
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            }
            else {
                let mut message = String::new();
                match output.status.code() {
                    Some(integer) => {
                        message.push_str(&format!("setx exitted with status code {}", integer));
                    }
                    None => {
                        // Shouldn't happen per docs, code() only returns None on unix.
                        message.push_str("The exit code for setx could not be determined.");
                    }
                }
                match String::from_utf8(output.stderr) {
                    Ok(utf8_stdout) => {
                        message.push_str("setx wrote the following to stderr:\n");
                        message.push_str(&utf8_stdout);
                    }
                    Err(_) => {
                        message.push_str("stderr content cannot be displayed because is not utf-8.")
                    }
                }

                Err(io::Error::new(io::ErrorKind::Other, message))
            }
        },
        Err(error) => Err(error)
    }
}
#[cfg(target_family = "unix")]
fn get_profile() -> io::Result<File> {
    dirs::home_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No home directory"))
        .and_then(find_profile)
}

#[cfg(target_family = "unix")]
fn find_profile(mut profile: PathBuf) -> io::Result<File> {
    profile.push(".bash_profile");
    let mut oo = OpenOptions::new();
    oo.append(true)
        .create(false);
    oo.open(profile.clone())
        .or_else(|_|{
            profile.pop();
            profile.push(".bash_login");
            oo.open(profile.clone())
        })
        .or_else(|_|{
            profile.pop();
            profile.push(".profile");
            oo.open(profile.clone())
        })
        .or_else(|_|{
            profile.pop();
            profile.push(".bash_profile");
            oo.create(true);
            oo.open(profile.clone())
        })
}
