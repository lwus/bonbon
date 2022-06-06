use {
    log::*,
    solana_sdk::clock::Slot,
    solana_transaction_status::UiTransactionEncoding,
};

#[derive(Debug)]
pub struct Config {
    psql_config: String,
    log_file: String,
}

async fn fetch(
    config: &Config,
    bigtable_path: String,
    block_range: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let re = regex::Regex::new(r"^(\d*)-(\d*)$")?;

    let (block_start, block_end) = (|| -> Option<(Slot, Slot)> {
        let caps = re.captures(block_range.as_str())?;
        let block_start = caps.get(1)?.as_str().parse::<Slot>().ok()?;
        let block_end = caps.get(2)?.as_str().parse::<Slot>().ok()?;
        if block_start > block_end {
            None
        } else {
            Some((block_start, block_end))
        }
    })().ok_or("Invalid --block_range")?;

    let (psql_client, psql_connection) = tokio_postgres::connect(
        config.psql_config.as_str(), tokio_postgres::NoTls).await?;

    let psql_join_handle = tokio::spawn(async move {
        if let Err(e) = psql_connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let insert_transaction_statement = psql_client.prepare(
        "INSERT INTO transactions VALUES ($1, $2, $3, $4)"
    ).await?;

    let bt = solana_storage_bigtable::LedgerStorage::new(
        true, None, Some(bigtable_path)).await.unwrap();

    // TODO: parameterize?
    let chunk_size = 16;
    let mut chunk_start = block_start;
    while chunk_start < block_end {
        let chunk_end = std::cmp::min(chunk_start + chunk_size, block_end);
        trace!("fetching slots {}..{}", chunk_start, chunk_end);

        let chunk_slots = bt.get_confirmed_blocks(
            chunk_start, (chunk_end - chunk_start) as usize).await?;

        for (slot, block) in bt.get_confirmed_blocks_with_data(&chunk_slots).await? {
            let slot = slot as i64;
            for (index, transaction) in block.transactions.into_iter().enumerate() {
                let index = index as i64;
                let mut found_token_or_metadata = false;
                for account_key in transaction.account_keys().iter() {
                    if *account_key == spl_token::id() || *account_key == mpl_token_metadata::id() {
                        found_token_or_metadata = true;
                        break;
                    }
                }
                if !found_token_or_metadata { continue; }

                // TODO: dedup some work in bigtable library?
                let signature = transaction.transaction_signature().clone();
                let serialized = bincode::serialize(
                    &transaction.encode(UiTransactionEncoding::Base64, None)?
                )?;

                psql_client.query(
                    &insert_transaction_statement,
                    &[
                        &slot,
                        &index,
                        &signature.as_ref(),
                        &serialized,
                    ],
                ).await?;
            }
        }

        chunk_start = chunk_end;
    }

    info!("finished block fetch. waiting for db join...");

    drop(psql_client);
    psql_join_handle.await?;

    Ok(())
}

fn partition(_config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let log_file_default = "bonbon.log";

    let matches = clap::Command::new(clap::crate_name!())
        .about(clap::crate_description!())
        .version(clap::crate_version!())
        .arg(
            clap::Arg::new("log_file")
                .long("log_file")
                .default_value(log_file_default)
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Log file")
        )
        .arg(
            clap::Arg::new("psql_config")
                .long("psql_config")
                .value_name("PSQL_CONFIG_STR")
                .takes_value(true)
                .global(true)
                .help("Transaction DB connection configuration")
        )
        .subcommand(
            clap::Command::new("fetch")
            .about("Fetch transactions into DB")
            .arg(
                clap::Arg::new("bigtable_path")
                    .long("bigtable_path")
                    .value_name("FILEPATH")
                    .takes_value(true)
                    .global(true)
                    .help("Path to bigtable credentials JSON")
            )
            .arg(
                clap::Arg::new("block_range")
                    .long("block_range")
                    .value_name("FILEPATH")
                    .takes_value(true)
                    .global(true)
                    .help("Block range to fetch")
            )
        )
        .subcommand(
            clap::Command::new("partition")
            .about("Partition all transactions found in the DB")
        )
        .get_matches();

    let config = Config {
        psql_config: matches
            .value_of("psql_config")
            .ok_or("Missing --psql_config")?
            .to_string(),
        log_file: matches
            .value_of("log_file")
            .unwrap()
            .to_string(),
    };

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().to_rfc3339(),
                record.level(),
                record.target(),
                message
            ))
        })
        // for most packages debug
        .level(log::LevelFilter::Debug)
        // we do a lot of logging at trace
        .level_for("chocolatier", log::LevelFilter::Trace)
        // postgres is a bit too verbose about queries so info
        .level_for("postgres", log::LevelFilter::Info)
        .level_for("tokio_postgres", log::LevelFilter::Info)
        .level_for("h2", log::LevelFilter::Info)
        .chain(fern::log_file(config.log_file.as_str())?)
        .apply()?;

    debug!("subcommand: {:?}", matches.subcommand());
    debug!("config: {:?}", config);

    match matches.subcommand() {
        Some(("fetch", sub_m)) => {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    fetch(
                        &config,
                        sub_m.value_of("bigtable_path")
                            .ok_or("Missing --bigtable_path")?.to_string(),
                        sub_m.value_of("block_range")
                            .ok_or("Missing --block_range")?.to_string(),
                    ).await
                })?
        }
        Some(("partition", _)) => {
            partition(&config)?;
        }
        o => {
            warn!("No matching subcommand found {:?}", o);
        }
    }

    Ok(())
}