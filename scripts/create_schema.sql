
CREATE TABLE transactions (
  slot BIGINT NOT NULL,
  block_index BIGINT NOT NULL,
  signature BYTEA NOT NULL,
  transaction BYTEA
);

CREATE TABLE partitions (
  partition_key BYTEA NOT NULL,
  program_key BYTEA NOT NULL,
  slot BIGINT NOT NULL,
  block_index BIGINT NOT NULL,
  outer_index BIGINT NOT NULL,
  inner_index BIGINT,
  signature BYTEA NOT NULL,
  instruction BYTEA
);

CREATE INDEX by_partition_key ON partitions (partition_key) ;

CREATE TYPE token_meta AS (
  account_index SMALLINT,
  mint_key BYTEA,
  owner_key BYTEA
);

CREATE TABLE account_keys (
  signature BYTEA PRIMARY KEY,
  keys BYTEA[],
  metas token_meta[]
);


CREATE TYPE edition_status AS enum (
  'none',
  'master',
  'limited'
);

CREATE TYPE limited_edition AS (
  master_key VARCHAR,
  -- u64 but close enough...
  edition_num BIGINT
);

CREATE TABLE bonbons (
  metadata_key VARCHAR NOT NULL,
  mint_key VARCHAR NOT NULL,
  mint_authority VARCHAR,
  current_owner VARCHAR,
  current_account VARCHAR,
  edition_status edition_status NOT NULL,
  limited_edition limited_edition
);

CREATE TYPE creator AS (
  creator_key VARCHAR,
  verified BOOLEAN,
  share SMALLINT
);

CREATE TABLE glazings (
  metadata_key VARCHAR NOT NULL,
  uri VARCHAR,
  collection_key VARCHAR,
  collection_verified BOOLEAN,
  creator0 creator,
  creator1 creator,
  creator2 creator,
  creator3 creator,
  creator4 creator,
  slot BIGINT NOT NULL,
  block_index BIGINT NOT NULL,
  outer_index BIGINT NOT NULL,
  inner_index BIGINT
);

CREATE TABLE transfers (
  mint_key VARCHAR NOT NULL,
  slot BIGINT NOT NULL,
  start_owner VARCHAR,
  start_account VARCHAR,
  end_owner VARCHAR,
  end_account VARCHAR
);

