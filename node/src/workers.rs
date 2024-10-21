// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Utilities and tests for locating the PVF worker binaries.

#![allow(dead_code)]

use is_executable::IsExecutable;
use polkadot_service::Error;
use std::path::PathBuf;

#[cfg(test)]
use std::sync::{Mutex, OnceLock};

/// Override the workers polkadot binary directory path, used for testing.
#[cfg(test)]
fn workers_exe_path_override() -> &'static Mutex<Option<PathBuf>> {
    static OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}
/// Override the workers lib directory path, used for testing.
#[cfg(test)]
fn workers_lib_path_override() -> &'static Mutex<Option<PathBuf>> {
    static OVERRIDE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

/// Determines the final set of paths to use for the PVF workers.
///
/// 1. Get the binaries from the workers path if it is passed in, or consider all possible
///    locations on the filesystem in order and get all sets of paths at which the binaries exist.
///
/// 2. If no paths exist, error out. We can't proceed without workers.
///
/// 3. Log a warning if more than one set of paths exists. Continue with the first set of paths.
///
/// 4. Check if the returned paths are executable. If not it's evidence of a borked installation
///    so error out.
///
/// 5. Do the version check, if mismatch error out.
///
/// 6. At this point the final set of paths should be good to use.
pub fn determine_workers_paths(
    given_workers_path: Option<PathBuf>,
    workers_names: Option<(String, String)>,
    node_version: Option<String>,
) -> Result<(PathBuf, PathBuf), Error> {
    let mut workers_paths = list_workers_paths(given_workers_path.clone(), workers_names.clone())?;
    if workers_paths.is_empty() {
        let current_exe_path = get_exe_path()?;
        return Err(Error::MissingWorkerBinaries {
            given_workers_path,
            current_exe_path,
            workers_names,
        });
    } else if workers_paths.len() > 1 {
        log::warn!("multiple sets of worker binaries found ({:?})", workers_paths,);
    }

    let (prep_worker_path, exec_worker_path) = workers_paths.swap_remove(0);
    if !prep_worker_path.is_executable() || !exec_worker_path.is_executable() {
        return Err(Error::InvalidWorkerBinaries { prep_worker_path, exec_worker_path });
    }

    // Do the version check.
    if let Some(node_version) = node_version {
        let worker_version = polkadot_node_core_pvf::get_worker_version(&prep_worker_path)?;
        if worker_version != node_version {
            return Err(Error::WorkerBinaryVersionMismatch {
                worker_version,
                node_version,
                worker_path: prep_worker_path,
            });
        }

        let worker_version = polkadot_node_core_pvf::get_worker_version(&exec_worker_path)?;
        if worker_version != node_version {
            return Err(Error::WorkerBinaryVersionMismatch {
                worker_version,
                node_version,
                worker_path: exec_worker_path,
            });
        }
    } else {
        log::warn!("Skipping node/worker version checks. This could result in incorrect behavior in PVF workers.");
    }

    Ok((prep_worker_path, exec_worker_path))
}

/// Get list of workers paths by considering the passed-in `given_workers_path` option, or possible
/// locations on the filesystem. See `new_full`.
fn list_workers_paths(
    given_workers_path: Option<PathBuf>,
    workers_names: Option<(String, String)>,
) -> Result<Vec<(PathBuf, PathBuf)>, Error> {
    if let Some(path) = given_workers_path {
        log::trace!("Using explicitly provided workers path {:?}", path);

        if path.is_executable() {
            return Ok(vec![(path.clone(), path)]);
        }

        let (prep_worker, exec_worker) = build_worker_paths(path, workers_names);

        // Check if both workers exist. Otherwise return an empty vector which results in an error.
        return if prep_worker.exists() && exec_worker.exists() {
            Ok(vec![(prep_worker, exec_worker)])
        } else {
            Ok(vec![])
        };
    }

    // Workers path not provided, check all possible valid locations.

    let mut workers_paths = vec![];

    // Consider the polkadot binary directory.
    {
        let exe_path = get_exe_path()?;

        let (prep_worker, exec_worker) =
            build_worker_paths(exe_path.clone(), workers_names.clone());

        // Add to set if both workers exist. Warn on partial installs.
        let (prep_worker_exists, exec_worker_exists) = (prep_worker.exists(), exec_worker.exists());
        if prep_worker_exists && exec_worker_exists {
            log::trace!("Worker binaries found at current exe path: {:?}", exe_path);
            workers_paths.push((prep_worker, exec_worker));
        } else if prep_worker_exists {
            log::warn!("Worker binary found at {:?} but not {:?}", prep_worker, exec_worker);
        } else if exec_worker_exists {
            log::warn!("Worker binary found at {:?} but not {:?}", exec_worker, prep_worker);
        }
    }

    // Consider the /usr/lib/polkadot/ directory.
    {
        #[allow(unused_mut)]
        let mut lib_path = PathBuf::from("/usr/lib/polkadot");
        #[cfg(test)]
        if let Some(ref path_override) = *workers_lib_path_override().lock().unwrap() {
            lib_path.clone_from(path_override);
        }

        let (prep_worker, exec_worker) = build_worker_paths(lib_path, workers_names);

        // Add to set if both workers exist. Warn on partial installs.
        let (prep_worker_exists, exec_worker_exists) = (prep_worker.exists(), exec_worker.exists());
        if prep_worker_exists && exec_worker_exists {
            log::trace!("Worker binaries found at /usr/lib/polkadot");
            workers_paths.push((prep_worker, exec_worker));
        } else if prep_worker_exists {
            log::warn!("Worker binary found at {:?} but not {:?}", prep_worker, exec_worker);
        } else if exec_worker_exists {
            log::warn!("Worker binary found at {:?} but not {:?}", exec_worker, prep_worker);
        }
    }

    Ok(workers_paths)
}

fn get_exe_path() -> Result<PathBuf, Error> {
    let mut exe_path = std::env::current_exe()?;
    let _ = exe_path.pop(); // executable file will always have a parent directory.
    #[cfg(test)]
    if let Some(ref path_override) = *workers_exe_path_override().lock().unwrap() {
        exe_path.clone_from(path_override);
    }
    Ok(exe_path)
}

fn build_worker_paths(
    worker_dir: PathBuf,
    workers_names: Option<(String, String)>,
) -> (PathBuf, PathBuf) {
    let (prep_worker_name, exec_worker_name) = workers_names.unwrap_or((
        polkadot_node_core_pvf::PREPARE_BINARY_NAME.to_string(),
        polkadot_node_core_pvf::EXECUTE_BINARY_NAME.to_string(),
    ));

    let mut prep_worker = worker_dir.clone();
    prep_worker.push(prep_worker_name);
    let mut exec_worker = worker_dir;
    exec_worker.push(exec_worker_name);

    (prep_worker, exec_worker)
}
