use clap::{Args, Parser, Subcommand, ValueEnum};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use synthetic_pop_core::{current_status, CoreEngine, PRODUCT_NAME, PROJECT_PROMISE};
use synthetic_pop_export::{export_forum_dataset, ExportFormat};
use synthetic_pop_scenarios::{
    generate_forum_dataset, parse_forum_config_yaml, ContentConfig, ForumScenarioConfig,
    PopulationConfig, ScenarioConfig,
};

#[derive(Debug, Parser)]
#[command(name = "synthetic-pop")]
#[command(about = PROJECT_PROMISE)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Generate(GenerateCommand),
}

#[derive(Debug, Args)]
struct GenerateCommand {
    #[command(subcommand)]
    scenario: GenerateScenario,
}

#[derive(Debug, Subcommand)]
enum GenerateScenario {
    Forum(ForumCommand),
}

#[derive(Debug, Args)]
struct ForumCommand {
    #[arg(long, value_name = "scenario.yml", conflicts_with_all = ["seed", "users", "communities", "posts", "comments"])]
    config: Option<PathBuf>,
    #[arg(long)]
    seed: Option<String>,
    #[arg(long)]
    users: Option<usize>,
    #[arg(long, value_delimiter = ',', value_name = "name[,name...]")]
    communities: Option<Vec<String>>,
    #[arg(long)]
    posts: Option<usize>,
    #[arg(long)]
    comments: Option<usize>,
    #[arg(long, value_enum)]
    format: Option<CliExportFormat>,
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliExportFormat {
    Json,
    Jsonl,
    Csv,
    SqliteSql,
    PostgresSql,
    PrismaSeed,
}

impl From<CliExportFormat> for ExportFormat {
    fn from(format: CliExportFormat) -> Self {
        match format {
            CliExportFormat::Json => Self::Json,
            CliExportFormat::Jsonl => Self::Jsonl,
            CliExportFormat::Csv => Self::Csv,
            CliExportFormat::SqliteSql => Self::SqliteSql,
            CliExportFormat::PostgresSql => Self::PostgresSql,
            CliExportFormat::PrismaSeed => Self::PrismaSeed,
        }
    }
}

#[derive(Debug)]
pub enum CliError {
    Arguments(clap::Error),
    MissingDirectFlag(&'static str),
    ReadConfig { path: PathBuf, source: io::Error },
    WriteOutput { path: PathBuf, source: io::Error },
    Config(synthetic_pop_scenarios::ScenarioConfigError),
    Generation(synthetic_pop_scenarios::ForumGenerationError),
    Export(synthetic_pop_export::ExportError),
}

impl CliError {
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Arguments(error) => error.exit_code(),
            _ => 1,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Arguments(error) => write!(formatter, "{error}"),
            Self::MissingDirectFlag(flag) => write!(
                formatter,
                "missing required flag --{flag}; provide --config or all direct generation flags"
            ),
            Self::ReadConfig { path, source } => {
                write!(
                    formatter,
                    "failed to read config file {}: {source}",
                    path.display()
                )
            }
            Self::WriteOutput { path, source } => {
                write!(
                    formatter,
                    "failed to write output file {}: {source}",
                    path.display()
                )
            }
            Self::Config(error) => write!(formatter, "{error}"),
            Self::Generation(error) => write!(formatter, "{error}"),
            Self::Export(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Arguments(error) => Some(error),
            Self::ReadConfig { source, .. } | Self::WriteOutput { source, .. } => Some(source),
            Self::Config(error) => Some(error),
            Self::Generation(error) => Some(error),
            Self::Export(error) => Some(error),
            Self::MissingDirectFlag(_) => None,
        }
    }
}

#[must_use]
pub fn status_message() -> String {
    let capabilities = CoreEngine::default().capabilities();
    let core_status = if capabilities.offline_core_available && capabilities.local_core_available {
        "Rust core foundation is available"
    } else {
        "Rust core foundation is not available"
    };
    let deterministic_rng_status = if capabilities.deterministic_rng_available {
        "deterministic RNG is available"
    } else {
        "deterministic RNG is not available"
    };
    let canonical_data_model_status = if capabilities.canonical_data_model_available {
        "canonical data model is available"
    } else {
        "canonical data model is not available"
    };
    format!(
        "{PRODUCT_NAME}: {PROJECT_PROMISE} ({status}).\n\
         {core_status}; {deterministic_rng_status}; {canonical_data_model_status}; Generation command available: {PRODUCT_NAME} generate forum.",
        status = current_status().label()
    )
}

/// Run the CLI with explicit output streams, making command behavior testable without spawning.
pub fn run_with_args<I, T>(
    args: I,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> Result<(), CliError>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::try_parse_from(args).map_err(CliError::Arguments)?;

    match cli.command {
        Some(Command::Generate(generate)) => run_generate(generate, stdout),
        None => {
            writeln!(stdout, "{}", status_message()).map_err(|source| CliError::WriteOutput {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
            writeln!(
                stderr,
                "Run `synthetic-pop generate forum --help` for generation options."
            )
            .map_err(|source| CliError::WriteOutput {
                path: PathBuf::from("<stderr>"),
                source,
            })
        }
    }
}

pub fn run() -> i32 {
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    match run_with_args(std::env::args_os(), &mut stdout, &mut stderr) {
        Ok(()) => 0,
        Err(CliError::Arguments(error)) => error
            .print()
            .map_or(error.exit_code(), |()| error.exit_code()),
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            error.exit_code()
        }
    }
}

fn run_generate(command: GenerateCommand, stdout: &mut impl Write) -> Result<(), CliError> {
    match command.scenario {
        GenerateScenario::Forum(command) => run_forum(command, stdout),
    }
}

fn run_forum(command: ForumCommand, stdout: &mut impl Write) -> Result<(), CliError> {
    let (config, format, output) = forum_config_format_and_output(command)?;
    let dataset = generate_forum_dataset(&config).map_err(CliError::Generation)?;
    let export = export_forum_dataset(&dataset, format).map_err(CliError::Export)?;

    if let Some(output) = output {
        fs::write(&output, export).map_err(|source| CliError::WriteOutput {
            path: output,
            source,
        })?;
    } else {
        stdout
            .write_all(export.as_bytes())
            .map_err(|source| CliError::WriteOutput {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
        stdout
            .write_all(b"\n")
            .map_err(|source| CliError::WriteOutput {
                path: PathBuf::from("<stdout>"),
                source,
            })?;
    }

    Ok(())
}

fn forum_config_format_and_output(
    command: ForumCommand,
) -> Result<(ForumScenarioConfig, ExportFormat, Option<PathBuf>), CliError> {
    let output = command.output;
    let (config, format) = if let Some(path) = command.config {
        let input = fs::read_to_string(&path).map_err(|source| CliError::ReadConfig {
            path: path.clone(),
            source,
        })?;
        let scenario = parse_forum_config_yaml(&input).map_err(CliError::Config)?;
        let ScenarioConfig::Forum(config) = scenario;
        let format = command
            .format
            .map_or_else(|| scenario_format(config.output_format), Into::into);
        (config, format)
    } else {
        let seed = require_flag(command.seed, "seed")?;
        let users = require_flag(command.users, "users")?;
        let communities = require_flag(command.communities, "communities")?;
        let posts = require_flag(command.posts, "posts")?;
        let comments = require_flag(command.comments, "comments")?;
        let config = ForumScenarioConfig {
            seed,
            population: PopulationConfig { users },
            communities,
            content: ContentConfig { posts, comments },
            output_format: synthetic_pop_scenarios::OutputFormat::Jsonl,
        };
        let format = command.format.map_or(ExportFormat::Jsonl, Into::into);
        (config, format)
    };

    Ok((config, format, output))
}

fn require_flag<T>(value: Option<T>, flag: &'static str) -> Result<T, CliError> {
    value.ok_or(CliError::MissingDirectFlag(flag))
}

fn scenario_format(format: synthetic_pop_scenarios::OutputFormat) -> ExportFormat {
    match format {
        synthetic_pop_scenarios::OutputFormat::Json => ExportFormat::Json,
        synthetic_pop_scenarios::OutputFormat::Jsonl => ExportFormat::Jsonl,
        synthetic_pop_scenarios::OutputFormat::Csv => ExportFormat::Csv,
        synthetic_pop_scenarios::OutputFormat::SqliteSql => ExportFormat::SqliteSql,
        synthetic_pop_scenarios::OutputFormat::PostgresSql => ExportFormat::PostgresSql,
        synthetic_pop_scenarios::OutputFormat::PrismaSeed => ExportFormat::PrismaSeed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn status_message_truthfully_reports_generation_command() {
        let message = status_message();

        assert!(message.contains("Rust core foundation is available"));
        assert!(message.contains("deterministic RNG is available"));
        assert!(message.contains("canonical data model is available"));
        assert!(message.contains("Generation command available"));
        assert!(!message.contains("Generation commands are not implemented yet"));
        assert!(message.contains("synthetic-pop generate forum"));
    }

    #[test]
    fn direct_forum_flags_generate_jsonl_to_stdout() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run_with_args(
            [
                "synthetic-pop",
                "generate",
                "forum",
                "--seed",
                "demo",
                "--users",
                "2",
                "--communities",
                "programming,gaming",
                "--posts",
                "2",
                "--comments",
                "2",
                "--format",
                "jsonl",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("generation should succeed");

        let output = String::from_utf8(stdout).expect("stdout should be utf8");
        assert!(output.contains("\"type\":\"user\""));
        assert!(output.contains("\"type\":\"activity_event\""));
        assert!(stderr.is_empty());
    }

    #[test]
    fn config_forum_generates_requested_format_to_output_file() {
        let dir = temp_dir();
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let config_path = dir.join("scenario.yml");
        let output_path = dir.join("forum.json");
        fs::write(
            &config_path,
            r#"
scenario: forum
seed: demo
population:
  users: 2
communities:
  - programming
content:
  posts: 1
  comments: 1
output_format: csv
"#,
        )
        .expect("config should be written");

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        run_with_args(
            [
                "synthetic-pop".to_string(),
                "generate".to_string(),
                "forum".to_string(),
                "--config".to_string(),
                config_path.display().to_string(),
                "--format".to_string(),
                "json".to_string(),
                "--output".to_string(),
                output_path.display().to_string(),
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect("generation should succeed");

        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
        let output = fs::read_to_string(output_path).expect("output should be written");
        assert!(output.contains("\"users\""));
        assert!(output.contains("\"activity_events\""));
    }

    #[test]
    fn missing_direct_flag_returns_useful_error() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let error = run_with_args(
            [
                "synthetic-pop",
                "generate",
                "forum",
                "--seed",
                "demo",
                "--users",
                "2",
                "--communities",
                "programming",
                "--posts",
                "1",
            ],
            &mut stdout,
            &mut stderr,
        )
        .expect_err("missing comments should fail");

        assert!(error
            .to_string()
            .contains("missing required flag --comments"));
        assert_eq!(error.exit_code(), 1);
    }

    fn temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("synthetic-pop-cli-test-{unique}"))
    }
}
