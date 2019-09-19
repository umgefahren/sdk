use crate::config::dfinity::Config;
use crate::config::{cache, dfx_version, is_debug};
use crate::lib::api_client::{Client, ClientConfig};
use crate::lib::error::DfxError::BuildError;
use crate::lib::error::{BuildErrorKind, DfxError, DfxResult};
use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

pub struct ActorScriptCommandBuilder<'a> {
    binary_path: PathBuf,
    input_path: Option<&'a str>,
    output_path: Option<&'a str>,
    idl: bool,
}

impl<'a> ActorScriptCommandBuilder<'a> {
    pub(self) fn new(version: &str) -> DfxResult<ActorScriptCommandBuilder> {
        Ok(ActorScriptCommandBuilder {
            binary_path: cache::get_binary_path_from_version(version, "asc")?,
            input_path: None,
            output_path: None,
            idl: false,
        })
    }

    pub fn input(&mut self, p: &'a Path) -> &mut Self {
        self.input_path = Some(p.to_str().unwrap());
        self
    }
    pub fn output(&mut self, p: &'a Path) -> &mut Self {
        self.output_path = Some(p.to_str().unwrap());
        self
    }
    pub fn build_idl(&mut self) -> &mut Self {
        self.idl = true;
        self
    }

    pub fn exec(&self) -> DfxResult {
        let mut cmd = Command::new(self.binary_path.clone());

        if let Some(ip) = self.input_path {
            cmd.arg(ip);
        }
        if let Some(op) = self.output_path {
            cmd.args(&["-o", op]);
        }
        if self.idl {
            cmd.arg("--idl");
        }

        let output = cmd.output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err = std::str::from_utf8(output.stderr.as_slice())?;
            Err(BuildError(BuildErrorKind::ActorScriptError(err.to_owned())))
        }
    }
}

pub struct IdlCompilerCommandBuilder<'a> {
    binary_path: PathBuf,
    input_path: Option<&'a str>,
    output_path: Option<&'a str>,
    js: bool,
}

impl<'a> IdlCompilerCommandBuilder<'a> {
    pub(self) fn new(version: &str) -> DfxResult<IdlCompilerCommandBuilder> {
        Ok(IdlCompilerCommandBuilder {
            binary_path: cache::get_binary_path_from_version(version, "didc")?,
            input_path: None,
            output_path: None,
            js: false,
        })
    }

    pub fn input(&mut self, p: &'a Path) -> &mut Self {
        self.input_path = Some(p.to_str().unwrap());
        self
    }
    pub fn output(&mut self, p: &'a Path) -> &mut Self {
        self.output_path = Some(p.to_str().unwrap());
        self
    }
    pub fn build_js(&mut self) -> &mut Self {
        self.js = true;
        self
    }

    pub fn exec(&self) -> DfxResult {
        let mut cmd = Command::new(self.binary_path.clone());

        if let Some(ip) = self.input_path {
            cmd.arg(ip);
        }
        if let Some(op) = self.output_path {
            cmd.args(&["-o", op]);
        }
        if self.js {
            cmd.arg("--js");
        }

        let output = cmd.output()?;
        if output.status.success() {
            Ok(())
        } else {
            let err = std::str::from_utf8(output.stderr.as_slice())?;
            Err(BuildError(BuildErrorKind::IdlCompilerError(err.to_owned())))
        }
    }
}

pub struct ClientCommandBuilder {
    nodemanager_path: PathBuf,
    client_path: PathBuf,
}

impl ClientCommandBuilder {
    pub(self) fn new(version: &str) -> DfxResult<ClientCommandBuilder> {
        Ok(ClientCommandBuilder {
            nodemanager_path: cache::get_binary_path_from_version(version, "nodemanager")?,
            client_path: cache::get_binary_path_from_version(version, "client")?,
        })
    }

    pub fn spawn(&self) -> DfxResult<Child> {
        Command::new(self.nodemanager_path.as_path())
            .arg(self.client_path.as_path())
            .spawn()
            .map_err(DfxError::from)
    }
}

pub trait BinaryMap {
    fn actorscript_compiler(&self) -> DfxResult<ActorScriptCommandBuilder>;
    fn idl_compiler(&self) -> DfxResult<IdlCompilerCommandBuilder>;
    fn client(&self) -> DfxResult<ClientCommandBuilder>;
}

pub struct VersionedBinaryMap {
    version: String,
}

impl BinaryMap for VersionedBinaryMap {
    fn actorscript_compiler(&self) -> DfxResult<ActorScriptCommandBuilder> {
        ActorScriptCommandBuilder::new(self.version.as_str())
    }
    fn idl_compiler(&self) -> DfxResult<IdlCompilerCommandBuilder> {
        IdlCompilerCommandBuilder::new(self.version.as_str())
    }
    fn client(&self) -> DfxResult<ClientCommandBuilder> {
        ClientCommandBuilder::new(self.version.as_str())
    }
}

/// An environment that contains the platform and general environment.
pub trait PlatformEnv {
    fn get_current_dir(&self) -> PathBuf;
}

/// An environment that manages the global binary cache.
pub trait BinaryCacheEnv {
    fn is_installed(&self) -> io::Result<bool>;
    fn install(&self) -> io::Result<()>;
}

/// An environment that can resolve binaries from the user-level cache.
pub trait BinaryResolverEnv {
    fn get_binary_map(&self) -> &'_ BinaryMap;
}

/// An environment that can get the project configuration.
pub trait ProjectConfigEnv {
    fn is_in_project(&self) -> bool;
    fn get_config(&self) -> Option<&Config>;
}

/// An environment that can create clients from environment.
pub trait ClientEnv {
    fn get_client(&self) -> Client;
}

/// An environment that can get the version of the DFX we should be using.
pub trait VersionEnv {
    fn get_version(&self) -> &String;
}

/// An environment that is inside a project.
pub struct InProjectEnvironment {
    version: String,
    config: Config,
    client: RefCell<Option<Client>>,
    binary_map: VersionedBinaryMap,
}

impl PlatformEnv for InProjectEnvironment {
    fn get_current_dir(&self) -> PathBuf {
        let config_path = self.get_config().unwrap().get_path();
        PathBuf::from(config_path.parent().unwrap())
    }
}

impl BinaryCacheEnv for InProjectEnvironment {
    fn is_installed(&self) -> io::Result<bool> {
        if is_debug() {
            // A debug version is NEVER installed (we always reinstall it).
            return Ok(false);
        }
        cache::is_version_installed(self.version.as_str())
    }
    fn install(&self) -> io::Result<()> {
        cache::install_version(self.version.as_str()).map(|_| ())
    }
}

impl<'a> BinaryResolverEnv for InProjectEnvironment {
    fn get_binary_map(&self) -> &'_ BinaryMap {
        &self.binary_map
    }
}

impl ProjectConfigEnv for InProjectEnvironment {
    fn is_in_project(&self) -> bool {
        true
    }
    fn get_config(&self) -> Option<&Config> {
        Some(&self.config)
    }
}

impl ClientEnv for InProjectEnvironment {
    fn get_client(&self) -> Client {
        {
            let mut cache = self.client.borrow_mut();
            if cache.is_some() {
                return cache.as_ref().unwrap().clone();
            }

            let start = self.config.get_config().get_defaults().get_start();
            let address = start.get_address("localhost");
            let port = start.get_port(8080);

            *cache = Some(Client::new(ClientConfig {
                url: format!("http://{}:{}", address, port),
            }));
        }

        // Have to recursively call ourselves to avoid cache getting out of scope.
        self.get_client()
    }
}

impl VersionEnv for InProjectEnvironment {
    fn get_version(&self) -> &String {
        &self.version
    }
}

impl InProjectEnvironment {
    pub fn from_current_dir() -> DfxResult<InProjectEnvironment> {
        let config = Config::from_current_dir()?;
        let version = config
            .get_config()
            .get_dfx()
            .unwrap_or_else(|| dfx_version().to_owned());

        Ok(InProjectEnvironment {
            version: version.clone(),
            config,
            client: RefCell::new(None),
            binary_map: VersionedBinaryMap { version },
        })
    }
}

pub struct GlobalEnvironment {
    version: String,
    binary_map: VersionedBinaryMap,
}

impl PlatformEnv for GlobalEnvironment {
    fn get_current_dir(&self) -> PathBuf {
        std::env::current_dir().unwrap()
    }
}

impl BinaryCacheEnv for GlobalEnvironment {
    fn is_installed(&self) -> io::Result<bool> {
        if is_debug() {
            // A debug version is NEVER installed (we always reinstall it).
            return Ok(false);
        }
        cache::is_version_installed(self.version.as_str())
    }
    fn install(&self) -> io::Result<()> {
        cache::install_version(self.version.as_str()).map(|_| ())
    }
}

impl BinaryResolverEnv for GlobalEnvironment {
    fn get_binary_map(&self) -> &'_ BinaryMap {
        &self.binary_map
    }
}

impl ProjectConfigEnv for GlobalEnvironment {
    fn is_in_project(&self) -> bool {
        false
    }
    fn get_config(&self) -> Option<&Config> {
        None
    }
}

impl ClientEnv for GlobalEnvironment {
    fn get_client(&self) -> Client {
        panic!();
    }
}

impl VersionEnv for GlobalEnvironment {
    fn get_version(&self) -> &String {
        &self.version
    }
}

impl GlobalEnvironment {
    pub fn from_current_dir() -> DfxResult<GlobalEnvironment> {
        let version = dfx_version().to_owned();
        Ok(GlobalEnvironment {
            version: version.clone(),
            binary_map: VersionedBinaryMap { version },
        })
    }
}
