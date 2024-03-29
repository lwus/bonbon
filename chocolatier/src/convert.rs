use {
    bonbon::assemble as bb,
    bonbon::partition as bp,
    postgres_types::*,
    solana_sdk::pubkey::Pubkey,
};

#[derive(Debug, ToSql)]
#[postgres(name = "edition_status")]
pub enum EditionStatus {
    // Edition has not been created. This state is used temporarily for every NFT we encounter
    // since the metadata must be created before the edition, but it could also be an...
    // - SFT
    // - NFT where mint auth is held by e.g cardinal
    #[postgres(name = "none")]
    None,

    #[postgres(name = "master")]
    Master,

    #[postgres(name = "limited")]
    Limited,
}

impl From<bb::EditionStatus> for EditionStatus {
    fn from(e: bb::EditionStatus) -> Self {
        match e {
            bb::EditionStatus::None => Self::None,
            bb::EditionStatus::Master => Self::Master,
            bb::EditionStatus::Limited => Self::Limited,
        }
    }
}


#[derive(Debug)]
pub struct SqlPubkey(pub Pubkey);

impl ToSql for SqlPubkey {
    fn to_sql(
        &self,
        _ty: &Type,
        w: &mut bytes::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        use bytes::BufMut;
        w.put_slice(self.0.as_ref());
        Ok(IsNull::No)
    }

    postgres_types::accepts!(BYTEA);

    postgres_types::to_sql_checked!();
}

impl<'a> FromSql<'a> for SqlPubkey {
    fn from_sql(
        _ty: &Type,
        raw: &'a [u8],
    ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        let fixed: [u8; 32] = raw.try_into()?;
        Ok(Self(Pubkey::new_from_array(fixed)))
    }

    postgres_types::accepts!(BYTEA);
}


#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "limited_edition")]
pub struct LimitedEdition {
    master_key: String,

    edition_num: Option<i64>,

    instruction_index: InstructionIndex,
}

impl From<bb::LimitedEdition> for LimitedEdition {
    fn from(e: bb::LimitedEdition) -> Self {
        Self {
            master_key: e.master_key.to_string(),
            edition_num: e.edition_num,
            instruction_index: InstructionIndex::from(e.instruction_index),
        }
    }
}


#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "token_meta")]
pub struct TransactionTokenMeta {
    pub account_index: i16,

    pub mint_key: SqlPubkey,

    pub owner_key: Option<SqlPubkey>,
}

impl From<bp::TransactionTokenMeta> for TransactionTokenMeta {
    fn from(m: bp::TransactionTokenMeta) -> Self {
        Self {
            account_index: m.account_index.into(),
            mint_key: SqlPubkey(m.mint_key),
            owner_key: m.owner_key.map(SqlPubkey),
        }
    }
}

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "creator")]
pub struct Creator {
    #[postgres(name = "creator_key")]
    pub address: String,

    pub verified: bool,

    pub share: i16,
}

impl From<&bb::Creator> for Creator {
    fn from(c: &bb::Creator) -> Self {
        Self {
            address: c.address.to_string(),
            verified: c.verified,
            share: c.share,
        }
    }
}

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "instruction_index")]
pub struct InstructionIndex {
    pub slot: i64,

    pub block_index: i64,

    pub outer_index: i64,

    pub inner_index: Option<i64>,
}

impl From<bb::InstructionIndex> for InstructionIndex {
    fn from(c: bb::InstructionIndex) -> Self {
        Self {
            slot: c.slot,
            block_index: c.block_index,
            outer_index: c.outer_index,
            inner_index: c.inner_index,
        }
    }
}

