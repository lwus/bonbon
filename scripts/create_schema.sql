
CREATE TABLE transactions (
  slot BIGINT NOT NULL,
  block_index BIGINT NOT NULL,
  signature BYTEA NOT NULL,
  transaction BYTEA
);

CREATE TABLE partition_failures (
  program_key BYTEA NOT NULL,
  slot BIGINT NOT NULL,
  block_index BIGINT NOT NULL,
  outer_index BIGINT NOT NULL,
  inner_index BIGINT,
  signature BYTEA NOT NULL,
  instruction BYTEA
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

CREATE TYPE instruction_index AS (
  slot BIGINT,
  block_index BIGINT,
  outer_index BIGINT,
  inner_index BIGINT
);

CREATE TYPE limited_edition AS (
  master_key VARCHAR,
  -- u64 but close enough...
  edition_num BIGINT,
  instruction_index instruction_index
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
  name VARCHAR,
  symbol VARCHAR,
  uri VARCHAR,
  collection_key VARCHAR,
  collection_verified BOOLEAN,
  creator0 creator,
  creator1 creator,
  creator2 creator,
  creator3 creator,
  creator4 creator,
  instruction_index instruction_index NOT NULL
);

CREATE TABLE transfers (
  mint_key VARCHAR NOT NULL,
  slot BIGINT NOT NULL,
  start_owner VARCHAR,
  start_account VARCHAR,
  end_owner VARCHAR,
  end_account VARCHAR
);

CREATE FUNCTION numeric2bytea(_n NUMERIC) RETURNS BYTEA AS $$
DECLARE
    _b BYTEA := '\x';
    _v INTEGER;
BEGIN
    WHILE _n > 0 LOOP
        _v := _n % 256;
        _b := SET_BYTE(('\x00' || _b),0,_v);
        _n := (_n-_v)/256;
    END LOOP;
    RETURN _b;
END;
$$ LANGUAGE PLPGSQL IMMUTABLE STRICT;

CREATE FUNCTION base58_decode (str TEXT) RETURNS BYTEA AS $$
DECLARE
  -- TODO: array indexed by ascii value of character
  alphabet VARCHAR(255) = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
  v NUMERIC = 0;
  c CHAR(1);
  p INT;
BEGIN
  FOR i IN 1..char_length(str) LOOP
    c := substring(str FROM i FOR 1);
    p := position(c IN alphabet);
    IF p = 0 THEN
      RAISE 'Illegal base58 character ''%'' in ''%''', c, str;
    END IF;
    v := (v * 58) + (p - 1);
  END LOOP;
  RETURN numeric2bytea(v);
END;
$$ LANGUAGE PLPGSQL IMMUTABLE STRICT;

