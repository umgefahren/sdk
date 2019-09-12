use crate::lib::env::BinaryResolverEnv;
use crate::lib::error::{DfxError, DfxResult};
use notify::{watcher, RecursiveMode, Watcher};
use std::borrow::Borrow;
use std::ops::Deref;
use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

fn build_and_notify<T>(
    env: &Box<dyn BinaryResolverEnv>,
    file_path: &Path,
    output_path: &Path,
    on_start: &Box<dyn Fn(&Path) -> () + Send + Sync>,
    on_done: &Box<dyn Fn(&Path) -> () + Send + Sync>,
    on_error: &Box<dyn Fn(DfxError) -> () + Send + Sync>,
) -> ()
where
    T: Sized + BinaryResolverEnv,
{
    on_start(file_path);

    #[warn(unused_must_use)]
    {
        build_file(env.deref(), file_path, output_path)
            .map(|()| on_done(output_path))
            .map_err(on_error);
    }
}

pub fn watch_file(
    env: Box<dyn BinaryResolverEnv>,
    file_path: &Path,
    output_root: &Path,
    on_start: Box<dyn Fn(&Path) -> () + Send + Sync>,
    on_done: Box<dyn Fn(&Path) -> () + Send + Sync>,
    on_error: Box<dyn Fn(DfxError) -> () + Send + Sync>,
) -> DfxResult<Sender<()>> {
    let (tx, rx) = channel();
    let (sender, receiver) = channel();

    // There's a better way to do this, e.g. with a single thread watching all files, but this
    // works great for a few files.
    let mut watcher = watcher(tx, Duration::from_secs(1))?;
    watcher.watch(file_path, RecursiveMode::NonRecursive)?;

    // Make actual clones of values to move them in the thread.
    let file_path: Box<Path> = Box::from(file_path);
    let output_root: Box<Path> = Box::from(output_root);

    thread::spawn(move || {
        let fp = file_path.borrow();
        let out = output_root.borrow();

        build_and_notify(&env, &fp, &out, &on_start, &on_done, &on_error);
        loop {
            if receiver.try_recv().is_ok() {
                break;
            }

            if rx.recv_timeout(Duration::from_millis(80)).is_ok() {
                build_and_notify(&env, &fp, &out, &on_start, &on_done, &on_error);
            }
        }

        // Ignore result from unwatch. Nothing we can do.
        #[allow(unused_must_use)]
        {
            watcher.unwatch(fp);
        }
    });

    Ok(sender)
}

pub fn build_file<'a, T>(env: &'a T, input_path: &'a Path, output_path: &'a Path) -> DfxResult
where
    T: BinaryResolverEnv,
{
    let output_wasm_path = output_path.with_extension("wasm");
    let output_idl_path = output_path.with_extension("did");
    let output_js_path = output_path.with_extension("js");

    env.get_binary_command("asc")?
        .arg(input_path)
        .arg("-o")
        .arg(&output_wasm_path)
        .output()?;
    env.get_binary_command("asc")?
        .arg("--idl")
        .arg(input_path)
        .arg("-o")
        .arg(&output_idl_path)
        .output()?;
    env.get_binary_command("didc")?
        .arg("--js")
        .arg(&output_idl_path)
        .arg("-o")
        .arg(output_js_path)
        .output()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::env::temp_dir;
    use std::fs;
    use std::io;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::process;

    #[test]
    /// Runs "echo" instead of the compiler to make sure the binaries are called in order
    /// with the good arguments.
    fn build_file_wasm() -> () {
        // Create a binary cache environment that just returns "echo", so we can test the STDOUT.
        struct TestEnv<'a> {
            out_file: &'a fs::File,
        }

        impl<'a> BinaryResolverEnv for TestEnv<'a> {
            fn get_binary_command_path(&self, _binary_name: &str) -> io::Result<PathBuf> {
                // This should not be used.
                panic!("get_binary_command_path should not be called.")
            }
            fn get_binary_command(&self, binary_name: &str) -> io::Result<process::Command> {
                let stdout = self.out_file.try_clone()?;
                let stderr = self.out_file.try_clone()?;

                let mut cmd = process::Command::new("echo");

                cmd.arg(binary_name)
                    .stdout(process::Stdio::from(stdout))
                    .stderr(process::Stdio::from(stderr));

                Ok(cmd)
            }
        }

        let temp_path = temp_dir().join("stdout").with_extension("txt");
        let mut out_file = fs::File::create(temp_path.clone()).expect("Could not create file.");
        let env = TestEnv {
            out_file: &out_file,
        };

        build_file(&env, Path::new("/in/file.as"), Path::new("/out/file.wasm"))
            .expect("Function failed.");

        out_file.flush().expect("Could not flush.");

        let mut s = String::new();
        fs::File::open(temp_path)
            .expect("Could not open temp file.")
            .read_to_string(&mut s)
            .expect("Could not read temp file.");

        assert_eq!(
            s.trim(),
            r#"asc /in/file.as -o /out/file.wasm
                asc --idl /in/file.as -o /out/file.did
                didc --js /out/file.did -o /out/file.js"#
                .replace("                ", "")
        );
    }
}
