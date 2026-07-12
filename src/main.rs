use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Args, Parser, Subcommand};
use links_logs_vault::{LogLevel, LogPayload, LogRecord, Query, StorageFormat, Vault};

#[derive(Debug, Parser)]
#[command(
    name = "links-logs-vault",
    about = "Store and search text or binary logs in an associative vault"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Append one text message or binary file.
    Ingest(IngestArgs),
    /// Search stored text records and structured fields.
    Search(SearchArgs),
    /// Count records by severity.
    Stats(StoreArgs),
}

#[derive(Debug, Clone, Args)]
struct StoreArgs {
    /// Snapshot path. Its `.links` sidecar contains the associative graph.
    #[arg(long)]
    db: PathBuf,

    /// Snapshot codec: text/lino or binary/llvb.
    #[arg(long, default_value = "text")]
    format: StorageFormat,
}

#[derive(Debug, Args)]
struct IngestArgs {
    #[command(flatten)]
    store: StoreArgs,

    /// Event severity.
    #[arg(long, default_value = "info")]
    level: LogLevel,

    /// Producer or logical logger name.
    #[arg(long, default_value = "default")]
    channel: String,

    /// UTF-8 event body. Conflicts with `--binary`.
    #[arg(long, conflicts_with = "binary")]
    message: Option<String>,

    /// Read arbitrary event bytes from this file. Conflicts with `--message`.
    #[arg(long, conflicts_with = "message")]
    binary: Option<PathBuf>,

    /// Unix timestamp; defaults to the current second.
    #[arg(long)]
    timestamp: Option<u64>,

    /// Searchable context in `key=value` form. May be repeated.
    #[arg(long = "field", value_parser = parse_field)]
    fields: Vec<(String, String)>,
}

#[derive(Debug, Args)]
struct SearchArgs {
    #[command(flatten)]
    store: StoreArgs,

    /// Exact channel filter.
    #[arg(long)]
    channel: Option<String>,

    /// Inclusive severity floor.
    #[arg(long)]
    min_level: Option<LogLevel>,

    /// Inclusive lower timestamp bound.
    #[arg(long)]
    from: Option<u64>,

    /// Inclusive upper timestamp bound.
    #[arg(long)]
    to: Option<u64>,

    /// Case-insensitive message substring.
    #[arg(long)]
    contains: Option<String>,

    /// Exact context match in `key=value` form. May be repeated.
    #[arg(long = "field", value_parser = parse_field)]
    fields: Vec<(String, String)>,

    /// Maximum number of results.
    #[arg(long)]
    limit: Option<usize>,
}

fn parse_field(value: &str) -> Result<(String, String), String> {
    let (key, value) = value
        .split_once('=')
        .ok_or_else(|| String::from("field must use key=value syntax"))?;
    if key.is_empty() {
        return Err(String::from("field key cannot be empty"));
    }
    Ok((key.to_owned(), value.to_owned()))
}

fn run(cli: Cli) -> Result<String, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Ingest(args) => ingest(args),
        Command::Search(args) => search(args),
        Command::Stats(args) => stats(&args),
    }
}

fn ingest(args: IngestArgs) -> Result<String, Box<dyn std::error::Error>> {
    let timestamp = args
        .timestamp
        .unwrap_or(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs());
    let payload = match (args.message, args.binary) {
        (Some(message), None) => LogPayload::Text(message),
        (None, Some(path)) => LogPayload::Binary(fs::read(path)?),
        (None, None) => return Err("either --message or --binary is required".into()),
        (Some(_), Some(_)) => unreachable!("clap enforces argument conflicts"),
    };
    let record = LogRecord {
        timestamp,
        level: args.level,
        channel: args.channel,
        payload,
        fields: args.fields.into_iter().collect(),
    };
    let mut vault = Vault::open(&args.store.db, args.store.format)?;
    let id = vault.append(record)?;
    Ok(format!("stored record {id}\n"))
}

fn search(args: SearchArgs) -> Result<String, Box<dyn std::error::Error>> {
    let vault = Vault::open(&args.store.db, args.store.format)?;
    let query = Query {
        channel: args.channel,
        min_level: args.min_level,
        from_timestamp: args.from,
        to_timestamp: args.to,
        contains: args.contains,
        fields: args.fields.into_iter().collect(),
        limit: args.limit,
    };
    let mut output = String::new();
    for entry in vault.search(&query) {
        let payload = match &entry.record.payload {
            LogPayload::Text(value) => value.clone(),
            LogPayload::Binary(value) => format!("<{} binary bytes>", value.len()),
        };
        let fields = format_fields(&entry.record.fields);
        writeln!(
            output,
            "{} {} {} {} {}{}",
            entry.id,
            entry.record.timestamp,
            entry.record.level,
            entry.record.channel,
            payload,
            fields
        )
        .expect("writing to a String cannot fail");
    }
    Ok(output)
}

fn stats(args: &StoreArgs) -> Result<String, Box<dyn std::error::Error>> {
    let vault = Vault::open(&args.db, args.format)?;
    let mut output = format!(
        "records={} associations={}\n",
        vault.len(),
        vault.association_count()
    );
    for (level, count) in vault.count_by_level() {
        writeln!(output, "{level}={count}").expect("writing to a String cannot fail");
    }
    Ok(output)
}

fn format_fields(fields: &BTreeMap<String, String>) -> String {
    fields
        .iter()
        .fold(String::new(), |mut output, (key, value)| {
            output.push(' ');
            output.push_str(key);
            output.push('=');
            output.push_str(value);
            output
        })
}

fn write_output(writer: &mut impl io::Write, output: &str) -> io::Result<()> {
    match writer
        .write_all(output.as_bytes())
        .and_then(|()| writer.flush())
    {
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = run(Cli::parse())?;
    write_output(&mut io::stdout(), &output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct BrokenPipeWriter;

    impl io::Write for BrokenPipeWriter {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn broken_pipe_is_a_clean_exit() {
        assert!(write_output(&mut BrokenPipeWriter, "record\n").is_ok());
    }

    #[test]
    fn field_parser_keeps_equals_in_values() {
        assert_eq!(
            parse_field("query=a=b"),
            Ok((String::from("query"), String::from("a=b")))
        );
    }
}
