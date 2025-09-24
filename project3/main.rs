use clap::{Arg, Command};
use std::fs;
use std::path::Path;
use tokio;
use anyhow::{Result, Context};

#[derive(Debug)]
struct Config {
    input_file: String,
    output_file: String,
    workers: usize,
    batch_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("cli-tool")
        .version("1.0")
        .author("hamisionesmus")
        .about("High-performance CLI processing tool")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .value_name("FILE")
                .help("Input file to process")
                .required(true),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("FILE")
                .help("Output file")
                .required(true),
        )
        .arg(
            Arg::new("workers")
                .short('w')
                .long("workers")
                .value_name("NUM")
                .help("Number of worker threads")
                .default_value("4"),
        )
        .arg(
            Arg::new("batch-size")
                .short('b')
                .long("batch-size")
                .value_name("SIZE")
                .help("Batch size for processing")
                .default_value("1000"),
        )
        .get_matches();

    let config = Config {
        input_file: matches.get_one::<String>("input").unwrap().clone(),
        output_file: matches.get_one::<String>("output").unwrap().clone(),
        workers: matches.get_one::<String>("workers").unwrap().parse()?,
        batch_size: matches.get_one::<String>("batch-size").unwrap().parse()?,
    };

    println!("🚀 Starting CLI tool with config: {:?}", config);

    // Validate input file
    if !Path::new(&config.input_file).exists() {
        anyhow::bail!("Input file does not exist: {}", config.input_file);
    }

    // Process the file
    process_file(&config).await?;

    println!("✅ Processing completed successfully!");
    Ok(())
}

async fn process_file(config: &Config) -> Result<()> {
    println!("📂 Reading input file: {}", config.input_file);

    let content = fs::read_to_string(&config.input_file)
        .with_context(|| format!("Failed to read input file: {}", config.input_file))?;

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    println!("📊 Processing {} lines with {} workers", total_lines, config.workers);

    // Simulate processing with progress
    let mut processed = 0;
    let mut results = Vec::new();

    for chunk in lines.chunks(config.batch_size) {
        let batch_results = process_batch(chunk).await?;
        results.extend(batch_results);
        processed += chunk.len();

        let progress = (processed as f64 / total_lines as f64 * 100.0) as u32;
        println!("📈 Progress: {}% ({}/{})", progress, processed, total_lines);
    }

    // Write results
    let output_content = results.join("\n");
    fs::write(&config.output_file, output_content)
        .with_context(|| format!("Failed to write output file: {}", config.output_file))?;

    println!("💾 Results written to: {}", config.output_file);
    Ok(())
}

async fn process_batch(lines: &[&str]) -> Result<Vec<String>> {
    // Simulate async processing
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    let results: Vec<String> = lines
        .iter()
        .map(|line| format!("PROCESSED: {}", line.to_uppercase()))
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_batch() {
        let input = vec!["hello", "world"];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(process_batch(&input)).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], "PROCESSED: HELLO");
        assert_eq!(result[1], "PROCESSED: WORLD");
    }
}