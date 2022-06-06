use {
    borsh::de::BorshDeserialize,
    mpl_token_metadata::{
        instruction::MetadataInstruction,
        pda::find_metadata_account,
        state::Creator as MplCreator,
        state::Collection as MplCollection,
    },
    solana_sdk::{
        pubkey::Pubkey,
        instruction::CompiledInstruction,
    },
    spl_token::instruction::{AuthorityType, TokenInstruction},
};

#[derive(Clone, Debug, PartialEq)]
pub enum EditionStatus {
    // Edition has not been created. This state is used temporarily for every NFT we encounter
    // since the metadata must be created before the edition, but it could also be an...
    // - SFT
    // - NFT where mint auth is held by e.g cardinal
    None,

    Master,

    Limited,
}

impl Default for EditionStatus {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug)]
pub struct LimitedEdition {
    pub master_key: Pubkey,

    // TODO: this is kind of a PITA to track correctly with the old method. Punt for now
    // (Option)
    pub edition_num: Option<i64>,
}

#[derive(Debug)]
pub struct Creator {
    pub address: Pubkey,

    pub verified: bool,

    pub share: i16,
}

impl From<MplCreator> for Creator {
    fn from(creator: MplCreator) -> Self {
        Self {
            address: creator.address,
            verified: creator.verified,
            share: creator.share.into(),
        }
    }
}

fn from_creators(
    creators: Option<Vec<MplCreator>>,
) -> Vec<Creator> {
    creators.unwrap_or(vec![]).into_iter().map(Creator::from).collect()
}

#[derive(Debug)]
pub struct Collection {
    pub address: Pubkey,

    pub verified: bool,
}

impl From<MplCollection> for Collection {
    fn from(creator: MplCollection) -> Self {
        Self {
            address: creator.key,
            verified: creator.verified,
        }
    }
}

#[derive(Default, Debug)]
pub struct Bonbon {
    pub mint_key: Pubkey, // could be pubkey::default

    pub metadata_key: Pubkey, // could be pubkey::default

    pub current_account: Option<Pubkey>,

    pub edition_status: EditionStatus,

    pub limited_edition: Option<LimitedEdition>,

    pub uri: Vec<u8>,

    pub creators: Vec<Creator>,

    pub collection: Option<Collection>,
}

#[derive(Debug)]
pub enum ErrorCode {
    BadAccountKeyIndex,

    FailedInstructionDeserialization,

    InvalidMetadataCreate,

    InvalidMetadataUpdate,

    InvalidMasterEditionCreate,

    // includes unverify creator/collection
    InvalidMetadataVerifyOperation,
}


pub fn update_metadata_instruction(
    bonbon: &mut Bonbon,
    instruction: &CompiledInstruction,
    account_keys: &[Pubkey],
) -> Result<(), ErrorCode> {
    let get_account_key = |index: usize| account_keys.get(
        usize::from(instruction.accounts[index])
    ).ok_or(ErrorCode::BadAccountKeyIndex);

    let metadata_instruction = MetadataInstruction::try_from_slice(&instruction.data)
        .map_err(|_| ErrorCode::FailedInstructionDeserialization)?;


    match metadata_instruction {
        MetadataInstruction::CreateMetadataAccount(args) => {
            // OG create metadata
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != *metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = *metadata_key;
            bonbon.uri = args.data.uri.into_bytes();
            bonbon.creators = from_creators(args.data.creators);
        },
        MetadataInstruction::CreateMetadataAccountV2(args) => {
            // create metadata with datav2 (adds collection info, etc)
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != *metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = *metadata_key;
            bonbon.uri = args.data.uri.into_bytes();
            bonbon.creators = from_creators(args.data.creators);
            bonbon.collection = args.data.collection.map(Collection::from);
        },
        MetadataInstruction::UpdateMetadataAccount(args) => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataUpdate);
            }

            if let Some(data) = args.data {
                bonbon.uri = data.uri.into_bytes();
                bonbon.creators = from_creators(data.creators);
            }
        },
        MetadataInstruction::UpdateMetadataAccountV2(args) => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataUpdate);
            }

            if let Some(data) = args.data {
                bonbon.uri = data.uri.into_bytes();
                bonbon.creators = from_creators(data.creators);
                bonbon.collection = data.collection.map(Collection::from);
            }
        },
        MetadataInstruction::DeprecatedCreateMasterEdition(_) => {
            // master edition with printing tokens (and reservation list?)
            let metadata_key = get_account_key(7)?;
            if bonbon.metadata_key != *metadata_key
                    || bonbon.edition_status != EditionStatus::None {
                return Err(ErrorCode::InvalidMasterEditionCreate);
            }

            bonbon.edition_status = EditionStatus::Master;
        }
        MetadataInstruction::CreateMasterEdition(_) => {
            // edition v2 w/ bitvec directly
            let metadata_key = get_account_key(5)?;
            if bonbon.metadata_key != *metadata_key
                    || bonbon.edition_status != EditionStatus::None {
                return Err(ErrorCode::InvalidMasterEditionCreate);
            }

            bonbon.edition_status = EditionStatus::Master;
        }
        MetadataInstruction::CreateMasterEditionV3(_) => {
            // not sure why this exists
            let metadata_key = get_account_key(5)?;
            if bonbon.metadata_key != *metadata_key
                    || bonbon.edition_status != EditionStatus::None {
                return Err(ErrorCode::InvalidMasterEditionCreate);
            }

            bonbon.edition_status = EditionStatus::Master;
        }
        MetadataInstruction::DeprecatedMintNewEditionFromMasterEditionViaPrintingToken => {
            // TODO: link with master edition for uri, creators, collection
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != *metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = *metadata_key;
            bonbon.edition_status = EditionStatus::Limited;
            bonbon.limited_edition = None;
        }
        MetadataInstruction::MintNewEditionFromMasterEditionViaToken(args) => {
            // TODO: link with master edition for uri, creators, collection
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != *metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = *metadata_key;
            bonbon.edition_status = EditionStatus::Limited;
            bonbon.limited_edition = Some(LimitedEdition {
                master_key: *get_account_key(10)?,
                edition_num: Some(args.edition as i64),
            });
        }
        MetadataInstruction::MintNewEditionFromMasterEditionViaVaultProxy(args) => {
            // TODO: link with master edition for uri, creators, collection
            let metadata_key = get_account_key(0)?;
            if find_metadata_account(&bonbon.mint_key).0 != *metadata_key {
                return Err(ErrorCode::InvalidMetadataCreate);
            }

            bonbon.metadata_key = *metadata_key;
            bonbon.edition_status = EditionStatus::Limited;
            bonbon.limited_edition = Some(LimitedEdition {
                master_key: *get_account_key(12)?,
                edition_num: Some(args.edition as i64),
            });
        }
        MetadataInstruction::SignMetadata => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let creator_key = get_account_key(1)?;
            for creator in &mut bonbon.creators {
                if creator.address == *creator_key {
                    creator.verified = true;
                    break;
                }
            }
        }
        MetadataInstruction::RemoveCreatorVerification => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            let creator_key = get_account_key(1)?;
            for creator in &mut bonbon.creators {
                if creator.address == *creator_key {
                    creator.verified = false;
                    break;
                }
            }
        }
        MetadataInstruction::VerifyCollection => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            if let Some(collection) = &mut bonbon.collection {
                collection.verified = true;
            } else {
                // TODO: metadata processor seems to accept this. keep stats?
            }
        }
        MetadataInstruction::SetAndVerifyCollection => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            bonbon.collection = Some(Collection {
                address: *get_account_key(4)?,
                verified: true,
            });
        }
        MetadataInstruction::UnverifyCollection => {
            let metadata_key = get_account_key(0)?;
            if bonbon.metadata_key != *metadata_key {
                return Err(ErrorCode::InvalidMetadataVerifyOperation);
            }

            if let Some(collection) = &mut bonbon.collection {
                collection.verified = false;
            } else {
                // TODO: metadata processor seems to accept this. keep stats?
            }
        }
        MetadataInstruction::UpdatePrimarySaleHappenedViaToken => { }
        MetadataInstruction::DeprecatedSetReservationList(_) => { }
        MetadataInstruction::DeprecatedCreateReservationList => { }
        MetadataInstruction::DeprecatedMintPrintingTokensViaToken(_) => { }
        MetadataInstruction::DeprecatedMintPrintingTokens(_) => { }
        MetadataInstruction::ConvertMasterEditionV1ToV2 => { }
        MetadataInstruction::PuffMetadata => { }
        MetadataInstruction::Utilize(_) => { }
        MetadataInstruction::ApproveUseAuthority(_) => { }
        MetadataInstruction::RevokeUseAuthority => { }
        MetadataInstruction::ApproveCollectionAuthority => { }
        MetadataInstruction::RevokeCollectionAuthority => { }
        MetadataInstruction::FreezeDelegatedAccount => { }
        MetadataInstruction::ThawDelegatedAccount => { }
    }

    Ok(())
}

pub fn update_token_instruction(
    bonbon: &mut Bonbon,
    instruction: &CompiledInstruction,
    account_keys: &[Pubkey],
) -> Result<(), ErrorCode> {
    let get_account_key = |index: usize| account_keys.get(
        usize::from(instruction.accounts[index])
    ).ok_or(ErrorCode::BadAccountKeyIndex);

    let token_instruction = TokenInstruction::unpack(&instruction.data)
        .map_err(|_| ErrorCode::FailedInstructionDeserialization)?;

    match token_instruction {
        TokenInstruction::InitializeMint { .. } => {
            bonbon.mint_key = *get_account_key(0)?;
        },
        // initializing an account doesn't change who currently owns it
        TokenInstruction::InitializeAccount { .. } => {},
        TokenInstruction::InitializeAccount2 { .. } => {},
        TokenInstruction::Transfer { .. } => {
            bonbon.current_account = Some(*get_account_key(1)?);
        }
        TokenInstruction::SetAuthority { authority_type, .. } => {
            match authority_type {
                AuthorityType::AccountOwner => {
                    // no account change. owner changes though possibly
                    // TODO
                }
                _ => {}
            }
        }
        TokenInstruction::MintTo { .. } => {
            bonbon.current_account = Some(*get_account_key(1)?);
        }
        TokenInstruction::Burn { .. } => {
            bonbon.current_account = None;
        }
        TokenInstruction::TransferChecked { .. } => {
            bonbon.current_account = Some(*get_account_key(2)?);
        }
        TokenInstruction::MintToChecked { .. } => {
            bonbon.current_account = Some(*get_account_key(1)?);
        }
        TokenInstruction::BurnChecked { .. } => {
            bonbon.current_account = None;
        }
        TokenInstruction::InitializeMultisig { .. } => {}
        TokenInstruction::Approve { .. } => {}
        TokenInstruction::Revoke => {}
        TokenInstruction::CloseAccount => {
            // mints can't be closed and a token account must have zero balance to be closed so...
        }
        TokenInstruction::FreezeAccount => {}
        TokenInstruction::ThawAccount => {}
        TokenInstruction::ApproveChecked { .. } => {}
        TokenInstruction::SyncNative => {}
    }

    Ok(())
}

pub struct BonbonUpdater {
    pub program_id: Pubkey,

    pub update: fn (
        bonbon: &mut Bonbon,
        instruction: &CompiledInstruction,
        account_keys: &[Pubkey],
    ) -> Result<(), ErrorCode>,
}

impl Bonbon {
    pub fn update(
        &mut self,
        instruction: &CompiledInstruction,
        account_keys: &[Pubkey],
        updaters: &[BonbonUpdater]
    ) -> Result<(), ErrorCode> {
        let program_id = account_keys.get(usize::from(instruction.program_id_index))
            .ok_or(ErrorCode::BadAccountKeyIndex)?;

        if let Some(BonbonUpdater { update, .. }) = updaters.iter().find(
                |u| u.program_id == *program_id) {
            update(self, instruction, account_keys)
        } else {
            Ok(())
        }
    }
}

