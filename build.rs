use std::{
    env, fs,
    path::{Path, PathBuf},
};

const DHT_FOLDER_PATH: &str = "dht";
const COPY_DIR: [&str; 1] = [DHT_FOLDER_PATH];

/// A helper function for recursively copying a directory.
fn copy_dir<P, Q>(from: P, to: Q)
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let to = to.as_ref().to_path_buf();
    for path in fs::read_dir(from).unwrap() {
        let path = path.unwrap().path();
        let to = to.clone().join(path.file_name().unwrap());
        if path.is_file() {
            fs::copy(&path, to).unwrap();
        } else if path.is_dir() {
            if !to.exists() {
                fs::create_dir(&to).unwrap();
            }
            copy_dir(&path, to);
        } else {
            /* Skip other content */
        }
    }
}

fn main() {
    init_folders();
    let out = env::var("PROFILE").unwrap();
    for dir in COPY_DIR {
        let out = PathBuf::from(format!("target/{}/{}", out, dir));
        if out.exists() {
            fs::remove_dir_all(&out).unwrap();
        }
        fs::create_dir(&out).unwrap();
        copy_dir(dir, &out);
    }
}

fn init_folders() {
    if !Path::new(DHT_FOLDER_PATH).exists() {
        fs::create_dir(DHT_FOLDER_PATH).expect("Can't create folder dht.");
    }
}
