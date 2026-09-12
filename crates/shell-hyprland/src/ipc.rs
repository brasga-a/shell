use std::{
    env,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    time::Duration,
};

use shell_core::CompositorError;

const IO_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyprlandPaths {
    runtime_dir: PathBuf,
    signature: String,
}

impl HyprlandPaths {
    pub fn new(
        runtime_dir: impl Into<PathBuf>,
        signature: impl Into<String>,
    ) -> Result<Self, CompositorError> {
        let runtime_dir = runtime_dir.into();
        let signature = signature.into();
        if signature.trim().is_empty() {
            return Err(CompositorError::Unavailable {
                message: "HYPRLAND_INSTANCE_SIGNATURE is empty".to_owned(),
            });
        }
        Ok(Self {
            runtime_dir,
            signature,
        })
    }

    pub fn from_environment() -> Result<Self, CompositorError> {
        let runtime_dir =
            env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| CompositorError::Unavailable {
                message: "XDG_RUNTIME_DIR is not set".to_owned(),
            })?;
        let signature =
            env::var("HYPRLAND_INSTANCE_SIGNATURE").map_err(|_| CompositorError::Unavailable {
                message: "HYPRLAND_INSTANCE_SIGNATURE is not set".to_owned(),
            })?;
        Self::new(runtime_dir, signature)
    }

    pub fn command_socket(&self) -> PathBuf {
        self.socket(".socket.sock")
    }

    pub fn event_socket(&self) -> PathBuf {
        self.socket(".socket2.sock")
    }

    fn socket(&self, name: &str) -> PathBuf {
        self.runtime_dir
            .join("hypr")
            .join(&self.signature)
            .join(name)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HyprlandIpc {
    paths: HyprlandPaths,
}

impl HyprlandIpc {
    pub(crate) fn new(paths: HyprlandPaths) -> Self {
        Self { paths }
    }

    pub(crate) fn request(&self, command: &str) -> Result<String, CompositorError> {
        let mut stream = UnixStream::connect(self.paths.command_socket()).map_err(|error| {
            CompositorError::Connection {
                message: format!("could not connect to Hyprland command socket: {error}"),
            }
        })?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .and_then(|_| stream.set_write_timeout(Some(IO_TIMEOUT)))
            .map_err(|error| CompositorError::Connection {
                message: format!("could not configure Hyprland command socket: {error}"),
            })?;
        stream
            .write_all(command.as_bytes())
            .map_err(|error| CompositorError::Connection {
                message: format!("could not write Hyprland command '{command}': {error}"),
            })?;

        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|error| CompositorError::Connection {
                message: format!("could not read Hyprland response for '{command}': {error}"),
            })?;
        Ok(response)
    }

    pub(crate) fn connect_events(&self) -> Result<UnixStream, CompositorError> {
        UnixStream::connect(self.paths.event_socket()).map_err(|error| {
            CompositorError::Connection {
                message: format!("could not connect to Hyprland event socket: {error}"),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::HyprlandPaths;

    #[test]
    fn builds_standard_socket_paths() {
        let paths = HyprlandPaths::new("/run/user/1000", "instance").expect("valid paths");

        assert_eq!(
            paths.command_socket(),
            std::path::Path::new("/run/user/1000/hypr/instance/.socket.sock")
        );
        assert_eq!(
            paths.event_socket(),
            std::path::Path::new("/run/user/1000/hypr/instance/.socket2.sock")
        );
    }

    #[test]
    fn rejects_empty_instance_signature() {
        assert!(HyprlandPaths::new("/run/user/1000", " ").is_err());
    }
}
