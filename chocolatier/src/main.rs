use {
    log::*,
    anyhow::{Result, anyhow},
    postgres::fallible_iterator::FallibleIterator,
    prost::Message,
    solana_sdk::{
        clock::Slot,
        instruction::CompiledInstruction,
        pubkey::Pubkey,
    },
    solana_storage_proto::convert::generated,
    solana_transaction_status::TransactionWithStatusMeta,
};

pub mod convert;

#[derive(Debug)]
pub struct Config {
    psql_config: String,
    log_file: String,
}

async fn fetch(
    config: &Config,
    bigtable_path: String,
    block_range: String,
) -> Result<()> {
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
    })().ok_or(anyhow!("Invalid --block_range"))?;

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
        let limit = std::cmp::min(chunk_size, block_end - chunk_start);

        let chunk_slots = bt.get_confirmed_blocks(
            chunk_start, limit as usize).await?;

        for (slot, block) in bt.get_confirmed_blocks_with_data(&chunk_slots).await? {
            let slot = slot as i64;
            for (index, transaction) in block.transactions.into_iter().enumerate() {
                // skip errors
                if transaction.get_status_meta().map(|m| m.status.is_err()) == Some(true) {
                    continue;
                }
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
                let protobuf_tx = generated::ConfirmedTransaction::from(transaction);
                let mut buf = Vec::with_capacity(protobuf_tx.encoded_len());
                protobuf_tx.encode(&mut buf).unwrap();
                // TODO: compress?

                psql_client.query(
                    &insert_transaction_statement,
                    &[
                        &slot,
                        &index,
                        &signature.as_ref(),
                        &buf,
                    ],
                ).await?;
            }
        }

        chunk_start = chunk_slots.last().unwrap_or(&block_end) + 1;
    }

    info!("finished block fetch. waiting for db join...");

    drop(psql_client);
    psql_join_handle.await?;

    Ok(())
}

fn partition(config: &Config) -> Result<()> {
    use bonbon::partition::*;
    let partitioners = [
        InstructionPartitioner {
            partitioner: partition_token_instruction,
            program_id: spl_token::id(),
        },
        InstructionPartitioner {
            partitioner: partition_metadata_instruction,
            program_id: mpl_token_metadata::id(),
        },
    ];

    let mut psql_client = postgres::Client::connect(
        config.psql_config.as_str(), postgres::NoTls)?;

    let select_all_statement = psql_client.prepare(
        "SELECT *
         FROM transactions
         ORDER BY (slot, block_index)
        ",
    )?;

    let mut insert_client = postgres::Client::connect(
        config.psql_config.as_str(), postgres::NoTls)?;

    let insert_transaction_statement = insert_client.prepare(
        "INSERT INTO partitions VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
    )?;

    let insert_account_keys_statement = insert_client.prepare(
        "INSERT INTO account_keys VALUES ($1, $2, $3)"
    )?;

    let params: &[&str] = &[];
    let query_start = std::time::Instant::now();
    let mut it = psql_client.query_raw(
        &select_all_statement,
        params,
    )?;
    log::info!("initial query took {:?}", query_start.elapsed());

    let loop_start = std::time::Instant::now();
    while let Some(row) = it.next()? {
        let slot: i64 = row.get(0);
        let block_index: i64 = row.get(1);
        let signature: Vec<u8> = row.get(2);
        let transaction: Vec<u8> = row.get(3);

        let transaction = generated::ConfirmedTransaction::decode(&transaction[..])?;
        let transaction = TransactionWithStatusMeta::try_from(transaction)?;

        // skip errors
        if transaction.get_status_meta().map(|m| m.status.is_err()) == Some(true) {
            continue;
        }

        let account_keys = transaction.account_keys()
            .iter().map(|k| k.as_ref().to_vec()).collect::<Vec<_>>();

        match partition_transaction(transaction, &partitioners) {
            Ok((partitioned, token_metas)) => {
                if partitioned.len() != 0 {
                    insert_client.query(
                        &insert_account_keys_statement,
                        &[
                            &signature.as_slice(),
                            &account_keys,
                            &token_metas.into_iter()
                                .map(|m| convert::TransactionTokenMeta::from(m))
                                .collect::<Vec<_>>(),
                        ],
                    )?;
                }
                for PartitionedInstruction {
                    instruction,
                    partition_key,
                    program_key,
                    outer_index,
                    inner_index,
                } in partitioned {
                    // TODO: soft error?
                    let serialized = bincode::serialize(&instruction)?;
                    insert_client.query(
                        &insert_transaction_statement,
                        &[
                            &partition_key.as_ref(),
                            &program_key.as_ref(),
                            &slot,
                            &block_index,
                            &outer_index,
                            &inner_index,
                            &signature.as_slice(),
                            &serialized,
                        ],
                    )?;
                }
            }
            Err(err) => {
                warn!("failed to partition {}.{:04x} [{}]: {:?}",
                      slot, block_index, bs58::encode(signature).into_string(), err);
            }
        }
    }
    log::info!("partitioned in {:?}", loop_start.elapsed());

    Ok(())
}

fn reassemble(config: &Config) -> Result<()> {
    use bonbon::assemble::*;
    let mut psql_client = postgres::Client::connect(
        config.psql_config.as_str(), postgres::NoTls)?;

    let mut partition_client = postgres::Client::connect(
        config.psql_config.as_str(), postgres::NoTls)?;

    let select_all_token_mints_statement = partition_client.prepare(
        "SELECT DISTINCT partition_key
         FROM partitions
         WHERE program_key = decode($1, 'base64')
        ",
    )?;

    let select_partition_key = psql_client.prepare(
        "SELECT p.signature, p.instruction, a.keys, a.metas,
                p.slot, p.block_index, p.outer_index, p.inner_index
         FROM partitions p JOIN account_keys a ON p.signature = a.signature
         WHERE partition_key = decode($1, 'base64')
            OR partition_key = decode($2, 'base64')
         ORDER BY (slot, block_index, outer_index, inner_index)
        ",
    )?;

    let insert_bonbon_statement = psql_client.prepare(
        "INSERT INTO bonbons VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )?;

    let insert_glazing_statement = psql_client.prepare(
        "INSERT INTO glazings VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
    )?;

    let insert_transfer_statement = psql_client.prepare(
        "INSERT INTO transfers VALUES ($1, $2, $3, $4, $5, $6)"
    )?;

    let spl_token_id_encoded = base64::encode(spl_token::id());
    let params: &[&str] = &[&spl_token_id_encoded];
    let query_start = std::time::Instant::now();
    let mut it = partition_client.query_raw(
        &select_all_token_mints_statement,
        params,
    )?;
    log::info!("initial query took {:?}", query_start.elapsed());

    let updaters = [
        BonbonUpdater {
            update: update_token_instruction,
            program_id: spl_token::id(),
        },
        BonbonUpdater {
            update: update_metadata_instruction,
            program_id: mpl_token_metadata::id(),
        },
    ];

    let loop_start = std::time::Instant::now();
    let mut partition_queries = std::time::Duration::ZERO;
    let mut update_queries = std::time::Duration::ZERO;
    let mut deserialization_duration = std::time::Duration::ZERO;
    while let Some(row) = it.next()? {
        let mint_key = Pubkey::new(row.get(0));
        let metadata_key = mpl_token_metadata::pda::find_metadata_account(&mint_key).0;

        let mint_key_encoded = base64::encode(&mint_key);
        let metadata_key_encoded = base64::encode(&metadata_key);
        let query_start = std::time::Instant::now();
        let instructions = psql_client.query(
            &select_partition_key,
            &[&mint_key_encoded, &metadata_key_encoded],
        )?;
        partition_queries += query_start.elapsed();

        let mut bonbon = Bonbon::default();
        let mut update_err = None;
        for row in instructions {
            let deserialization_start = std::time::Instant::now();
            let instruction = bincode::deserialize
                ::<CompiledInstruction>(&row.get::<_, Vec<u8>>(1))?;

            let keys: Vec<convert::SqlPubkey> = row.get(2);
            let keys = keys.into_iter().map(|k| k.0).collect::<Vec<_>>();

            let metas: Vec<convert::TransactionTokenMeta> = row.get(3);
            let metas = metas.into_iter().map(|m| TransactionTokenOwnerMeta {
                account_index: m.account_index as u8, // TODO: check?
                owner_key: m.owner_key.0,
            }).collect::<Vec<_>>();
            deserialization_duration += deserialization_start.elapsed();

            let slot: i64 = row.get(4);
            let block_index: i64 = row.get(5);
            let outer_index: i64 = row.get(6);
            let inner_index: Option<i64> = row.get(7);

            let instruction_context = InstructionContext {
                account_keys: &keys,
                instruction: &instruction,
                owners: &metas,
                instruction_index: InstructionIndex { slot, block_index, outer_index, inner_index },
            };

            match bonbon.update(instruction_context, &updaters) {
                Ok(_) => {}
                Err(err) => {
                    update_err = Some(err);
                    break;
                }
            }
        }

        if let Some(err) = update_err {
            warn!("failed to make bonbon {}: {:?}",
                  mint_key, err);
            continue;
        }

        if bonbon.metadata_key == Pubkey::default() {
            continue;
        }

        // TODO: more verification on partition_keys?
        let query_start = std::time::Instant::now();
        psql_client.query(
            &insert_bonbon_statement,
            &[
                &bonbon.metadata_key.to_string(),
                &bonbon.mint_key.to_string(),
                &bonbon.mint_authority.to_string(),
                &bonbon.current_owner.as_ref().map(|k| k.owner.to_string()),
                &bonbon.current_owner.as_ref().map(|k| k.account.to_string()),
                &convert::EditionStatus::from(bonbon.edition_status),
                &bonbon.limited_edition.map(convert::LimitedEdition::from),
            ],
        )?;

        for glazing in bonbon.glazings {
            psql_client.query(
                &insert_glazing_statement,
                &[
                    &bonbon.metadata_key.to_string(),
                    &glazing.uri,
                    &glazing.collection.as_ref().map(|c| c.address.to_string()),
                    &glazing.collection.as_ref().map(|c| c.verified),
                    &glazing.creators.get(0).map(convert::Creator::from),
                    &glazing.creators.get(1).map(convert::Creator::from),
                    &glazing.creators.get(2).map(convert::Creator::from),
                    &glazing.creators.get(3).map(convert::Creator::from),
                    &glazing.creators.get(4).map(convert::Creator::from),
                    &glazing.instruction_index.slot,
                    &glazing.instruction_index.block_index,
                    &glazing.instruction_index.outer_index,
                    &glazing.instruction_index.inner_index,
                ],
            )?;
        }

        for transfer in bonbon.transfers {
            psql_client.query(
                &insert_transfer_statement,
                &[
                    &bonbon.mint_key.to_string(),
                    &transfer.slot,
                    &transfer.start.as_ref().map(|t| t.owner.to_string()),
                    &transfer.start.as_ref().map(|t| t.account.to_string()),
                    &transfer.end.as_ref().map(|t| t.owner.to_string()),
                    &transfer.end.as_ref().map(|t| t.account.to_string()),
                ],
            )?;
        };

        update_queries += query_start.elapsed();
    }
    log::info!("reassembled in {:?}", loop_start.elapsed());
    log::info!("partition queries took {:?}", partition_queries);
    log::info!("update queries took {:?}", update_queries);
    log::info!("deserialization marshalling took {:?}", deserialization_duration);

    Ok(())
}

fn main() -> Result<()> {
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
        .subcommand(
            clap::Command::new("reassemble")
            .about("Reassemble all partitioned keys found in the DB")
        )
        .get_matches();

    let config = Config {
        psql_config: matches
            .value_of("psql_config")
            .ok_or(anyhow!("Missing --psql_config"))?
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
        .level_for("bonbon", log::LevelFilter::Trace)
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
                            .ok_or(anyhow!("Missing --bigtable_path"))?.to_string(),
                        sub_m.value_of("block_range")
                            .ok_or(anyhow!("Missing --block_range"))?.to_string(),
                    ).await
                })?
        }
        Some(("partition", _)) => {
            partition(&config)?;
        }
        Some(("reassemble", _)) => {
            reassemble(&config)?;
        }
        o => {
            warn!("No matching subcommand found {:?}", o);
        }
    }

    Ok(())
}
