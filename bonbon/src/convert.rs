use {
    solana_sdk::{
        hash::Hash,
        instruction::CompiledInstruction,
        message::{VersionedMessage, Message, v0},
        pubkey::Pubkey,
        signature::Signature,
        transaction::VersionedTransaction,
    },
    solana_transaction_status::{
        EncodedTransaction,
        EncodedTransactionWithStatusMeta,
        InnerInstructions,
        TransactionStatusMeta,
        TransactionTokenBalance,
        TransactionWithStatusMeta,
        UiCompiledInstruction,
        UiLoadedAddresses,
        UiMessage,
        UiAddressTableLookup,
        UiInnerInstructions,
        UiInstruction,
        UiTransaction,
        UiTransactionStatusMeta,
        UiTransactionTokenBalance,
        VersionedTransactionWithStatusMeta,
    },
    std::str::FromStr,
};

pub enum ConversionError {
    BinaryDecodingFailed,

    SignatureDecodingFailed,

    HashDecodingFailed,

    PubkeyDecodingFailed,

    DataDecodingFailed,

    UnexpectedUiParsed,

    NonLegacyMissingMeta,
}

pub fn convert_hash(s: &str) -> Result<Hash, ConversionError> {
    Hash::from_str(s).map_err(|_| ConversionError::HashDecodingFailed)
}

pub fn convert_key(s: &str) -> Result<Pubkey, ConversionError> {
    Pubkey::from_str(s).map_err(|_| ConversionError::PubkeyDecodingFailed)
}

pub fn convert_keys(keys: Vec<String>) -> Result<Vec<Pubkey>, ConversionError> {
    keys.into_iter()
        .map(|s| convert_key(s.as_str()))
        .collect()
}

pub fn convert_instruction(
    UiCompiledInstruction { program_id_index, accounts, data }: UiCompiledInstruction,
) -> Result<CompiledInstruction, ConversionError> {
    Ok(CompiledInstruction {
        program_id_index,
        accounts,
        data: bs58::decode(data).into_vec().map_err(|_| ConversionError::DataDecodingFailed)?,
    })
}

pub fn convert_instructions(
    instructions: Vec<UiCompiledInstruction>,
) -> Result<Vec<CompiledInstruction>, ConversionError> {
    instructions.into_iter()
        .map(convert_instruction)
        .collect()
}

pub fn convert_inner_instructions(
    UiInnerInstructions { index, instructions }: UiInnerInstructions,
) -> Result<InnerInstructions, ConversionError> {
    Ok(InnerInstructions {
        index,
        instructions: instructions.into_iter()
            .map(|i| match i {
                UiInstruction::Parsed(_) => Err(ConversionError::UnexpectedUiParsed),
                UiInstruction::Compiled(compiled) => convert_instruction(compiled),
            }).collect::<Result<Vec<_>, _>>()?,
    })
}

pub fn convert_table_lookup(
    UiAddressTableLookup {
        account_key,
        writable_indexes,
        readonly_indexes,
    }: UiAddressTableLookup,
) -> Result<v0::MessageAddressTableLookup, ConversionError> {
    Ok(v0::MessageAddressTableLookup {
        account_key: convert_key(account_key.as_str())?,
        writable_indexes,
        readonly_indexes,
    })
}

pub fn convert_token_balance(
    UiTransactionTokenBalance { account_index, mint, ui_token_amount, owner }: UiTransactionTokenBalance,
) -> Result<TransactionTokenBalance, ConversionError> {
    Ok(TransactionTokenBalance {
        account_index,
        mint,
        ui_token_amount,
        owner: owner.unwrap_or(String::new()),
    })
}

pub fn convert_loaded_addresses(
    UiLoadedAddresses { writable, readonly }: UiLoadedAddresses,
) -> Result<v0::LoadedAddresses, ConversionError> {
    Ok(v0::LoadedAddresses {
        writable: convert_keys(writable)?,
        readonly: convert_keys(readonly)?,
    })
}

pub fn convert_ui_transaction(
    ui: UiTransaction,
) -> Result<VersionedTransaction, ConversionError> {
    Ok(VersionedTransaction {
        signatures: ui.signatures.into_iter()
            .map(|s| Signature::from_str(s.as_str()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ConversionError::SignatureDecodingFailed)?,
        message: match ui.message {
            UiMessage::Parsed(_) => return Err(ConversionError::UnexpectedUiParsed),
            UiMessage::Raw(raw) => {
                match raw.address_table_lookups {
                    None => VersionedMessage::Legacy(Message {
                        header: raw.header,
                        account_keys: convert_keys(raw.account_keys)?,
                        recent_blockhash: convert_hash(raw.recent_blockhash.as_str())?,
                        instructions: convert_instructions(raw.instructions)?,
                    }),
                    Some(lookups) => VersionedMessage::V0(v0::Message {
                        header: raw.header,
                        account_keys: convert_keys(raw.account_keys)?,
                        recent_blockhash: convert_hash(raw.recent_blockhash.as_str())?,
                        instructions: convert_instructions(raw.instructions)?,
                        address_table_lookups: lookups.into_iter().map(convert_table_lookup).collect::<Result<Vec<_>, _>>()?,
                    }),
                }
            }
        }
    })
}

pub fn convert_transaction_status_meta(
    meta: UiTransactionStatusMeta,
) -> Result<TransactionStatusMeta, ConversionError> {
    Ok(TransactionStatusMeta {
        status: meta.status,
        fee: meta.fee,
        pre_balances: meta.pre_balances,
        post_balances: meta.post_balances,
        inner_instructions: meta
            .inner_instructions
            .map(|ixs| ixs.into_iter().map(convert_inner_instructions).collect())
            .transpose()?,
        log_messages: meta.log_messages,
        pre_token_balances: meta
            .pre_token_balances
            .map(|balance| balance.into_iter().map(convert_token_balance).collect())
            .transpose()?,
        post_token_balances: meta
            .post_token_balances
            .map(|balance| balance.into_iter().map(convert_token_balance).collect())
            .transpose()?,
        rewards: meta.rewards,
        loaded_addresses: meta
            .loaded_addresses
            .map(|addresses| convert_loaded_addresses(addresses))
            .transpose()?
            .unwrap_or(v0::LoadedAddresses::default()),
    })
}

pub fn convert(
    tx: EncodedTransactionWithStatusMeta,
) -> Result<TransactionWithStatusMeta, ConversionError> {
    let transaction = match tx.transaction {
        EncodedTransaction::Json(ui) => convert_ui_transaction(ui)?,
        EncodedTransaction::LegacyBinary(_) => tx.transaction.decode().ok_or(ConversionError::BinaryDecodingFailed)?,
        EncodedTransaction::Binary(_, _) => tx.transaction.decode().ok_or(ConversionError::BinaryDecodingFailed)?,
    };

    match tx.meta {
        None => Ok(TransactionWithStatusMeta::MissingMetadata(
            // TODO
            transaction.into_legacy_transaction().ok_or(ConversionError::NonLegacyMissingMeta)?,
        )),
        Some(meta) => {
            let meta = convert_transaction_status_meta(meta)?;

            Ok(TransactionWithStatusMeta::Complete(
                VersionedTransactionWithStatusMeta {
                    transaction,
                    meta,
                }
            ))
        }
    }
}

