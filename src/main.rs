use std::{
    cmp::max,
    fs::File,
    io::{BufReader, Read},
    path::PathBuf,
    sync::Arc,
};

use arrow::{
    array::{Array, BinaryArray, BooleanArray, RecordBatch, StringViewArray, UInt64Array},
    datatypes::{DataType, Field, Schema},
};
use bitcoin::{
    consensus::Decodable,
    hashes::Hash,
    opcodes::all::{OP_CHECKSIG, OP_DUP, OP_EQUAL, OP_EQUALVERIFY, OP_HASH160},
    script::{PushBytesBuf, PushBytesError},
    BlockHash, PublicKey, ScriptBuf, Txid, VarInt,
};
use clap::Parser;
use log::*;
use parquet::{
    arrow::ArrowWriter,
    basic::{Compression, Encoding, ZstdLevel},
    file::properties::{EnabledStatistics, WriterProperties},
    format::SortingColumn,
};

type Error = Box<dyn std::error::Error>;

#[derive(Parser)]
#[command(version, about, long_about = None)]
/// Convert Bitcoin UTXO to DuckDB.
struct Args {
    /// Bitcoin UTXO file to read.
    #[arg(short = 'i')]
    input: PathBuf,

    /// Path to Parquet file to write.
    #[arg(short = 'o')]
    output: PathBuf,
}

/// Bitcoin Core varint != `rust-bitcoin`` VarInt
fn decode_varint<R: bitcoin::io::Read>(r: &mut R) -> Result<u64, Error> {
    let mut n = 0u64;
    // TODO: add checks
    let mut buf = [0u8; 1];
    loop {
        r.read_exact(&mut buf)?;
        let b = buf[0];
        n = (n << 7) | (b & 0x7F) as u64;
        if b & 0x80 != 0 {
            n += 1;
        } else {
            return Ok(n);
        }
    }
}

struct Coin {
    height: u64,
    coinbase: bool,
    amount: u64,
    script: ScriptBuf,
}

fn decode_coin<R: bitcoin::io::Read>(r: &mut R) -> Result<Coin, Error> {
    let code = decode_varint(r)?;
    let height = code >> 1;
    let coinbase = (code & 1) != 0;
    let amount = decompress_amount(decode_varint(r)?);

    let script = decode_script(r)?;
    Ok(Coin {
        height,
        coinbase,
        amount,
        script,
    })
}

fn decompress_amount(mut x: u64) -> u64 {
    // x = 0  OR  x = 1+10*(9*n + d - 1) + e  OR  x = 1+10*(n - 1) + 9
    if x == 0 {
        return 0;
    }
    x -= 1;
    // x = 10*(9*n + d - 1) + e
    let mut e = x % 10;
    x /= 10;
    let mut n;
    if e < 9 {
        // x = 9*n + d - 1
        let d = (x % 9) + 1;
        x /= 9;
        // x = n
        n = x * 10 + d;
    } else {
        n = x + 1;
    }
    while e > 0 {
        n *= 10;
        e -= 1;
    }
    n
}

const SPECIAL_SCRIPTS: usize = 6;

fn decode_script<R: bitcoin::io::Read>(r: &mut R) -> Result<ScriptBuf, Error> {
    let len: usize = decode_varint(r)?.try_into()?;
    Ok(if len < SPECIAL_SCRIPTS {
        let script_type = len as u8;
        let size = match script_type {
            0 | 1 => 20,
            2..=5 => 32,
            _ => unreachable!(),
        };
        let mut compressed = vec![0u8; size];
        r.read_exact(&mut compressed)?;
        decompress_script(script_type, compressed)?
    } else {
        let size = len - SPECIAL_SCRIPTS;
        let mut buf = vec![0u8; size];
        r.read_exact(&mut buf)?;
        ScriptBuf::from_bytes(buf)
    })
}

fn decompress_script(script_type: u8, mut bytes: Vec<u8>) -> Result<ScriptBuf, Error> {
    let builder = bitcoin::blockdata::script::Builder::new();
    let script = match script_type {
        0 => builder
            .push_opcode(OP_DUP)
            .push_opcode(OP_HASH160)
            .push_slice(into_push_bytes(bytes)?)
            .push_opcode(OP_EQUALVERIFY)
            .push_opcode(OP_CHECKSIG),
        1 => builder
            .push_opcode(OP_HASH160)
            .push_slice(into_push_bytes(bytes)?)
            .push_opcode(OP_EQUAL),
        2 | 3 => {
            bytes.insert(0, script_type);
            builder
                .push_slice(into_push_bytes(bytes)?)
                .push_opcode(OP_CHECKSIG)
        }
        4 | 5 => {
            bytes.insert(0, script_type - 2);
            let mut pubkey = PublicKey::from_slice(&bytes)?;
            pubkey.compressed = false;
            builder.push_key(&pubkey).push_opcode(OP_CHECKSIG)
        }
        _ => unreachable!(),
    }
    .into_script();
    assert!(script.is_p2pk() || script.is_p2pkh() || script.is_p2sh());
    Ok(script)
}

fn into_push_bytes(bytes: Vec<u8>) -> Result<PushBytesBuf, PushBytesError> {
    bytes.try_into()
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    env_logger::builder().format_timestamp_micros().init();

    let mut input = BufReader::new(File::open(args.input)?);
    let mut bytes = [0u8; 5];
    input.read_exact(&mut bytes)?;
    assert_eq!(&bytes, b"utxo\xFF");

    let mut bytes = [0u8; 2];
    input.read_exact(&mut bytes)?;
    let version = u16::from_le_bytes(bytes);

    let mut bytes = [0u8; 4];
    input.read_exact(&mut bytes)?;
    let network_magic: bitcoin::network::Network =
        bitcoin::p2p::Magic::from_bytes(bytes).try_into()?;

    let mut bytes = [0u8; 32];
    input.read_exact(&mut bytes)?;
    let block_hash = BlockHash::from_byte_array(bytes);

    let mut bytes = [0u8; 8];
    input.read_exact(&mut bytes)?;
    let num_utxos = u64::from_le_bytes(bytes);

    info!(
        "{:?} UTXO snapshot at block hash {}, contains {} coins, version {}",
        network_magic, block_hash, num_utxos, version
    );

    let mut batch = Batch::default();
    let schema = Arc::new(Schema::new(vec![
        Field::new("txid", DataType::Utf8View, false),
        Field::new("vout", DataType::UInt64, false),
        Field::new("height", DataType::UInt64, false),
        Field::new("coinbase", DataType::Boolean, false),
        Field::new("amount", DataType::UInt64, false),
        Field::new("script", DataType::Binary, false),
    ]));

    let output = File::create(args.output)?;
    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .set_max_row_group_size(64 * 1024)
        .set_sorting_columns(Some(vec![SortingColumn::new(5, false, false)]))
        .set_column_statistics_enabled("script".into(), EnabledStatistics::Page)
        .set_column_encoding("script".into(), Encoding::DELTA_BYTE_ARRAY)
        .build();
    let mut writer = ArrowWriter::try_new(output, schema.clone(), Some(props))?;

    let mut coins_per_hash_left = 0;
    let mut prevout_hash = Txid::all_zeros();
    let mut max_height = 0;
    for i in 1..=num_utxos {
        if coins_per_hash_left == 0 {
            prevout_hash = Decodable::consensus_decode(&mut input)?;
            coins_per_hash_left = VarInt::consensus_decode(&mut input)?.0;
            assert!(coins_per_hash_left > 0);
        }
        let prevout_index = VarInt::consensus_decode(&mut input)?.0;
        let coin = decode_coin(&mut input)?;
        max_height = max(max_height, coin.height);
        coins_per_hash_left -= 1;

        batch.txids.push(prevout_hash.to_string());
        batch.vouts.push(prevout_index);
        batch.heights.push(coin.height);
        batch.coinbases.push(coin.coinbase);
        batch.amounts.push(coin.amount);
        batch.scripts.push(coin.script);

        if i % 10_000_000 == 0 || i == num_utxos {
            let arrays: Vec<Arc<dyn Array>> = vec![
                Arc::new(StringViewArray::from_iter_values(&batch.txids)),
                Arc::new(UInt64Array::from(batch.vouts.clone())),
                Arc::new(UInt64Array::from(batch.heights.clone())),
                Arc::new(BooleanArray::from(batch.coinbases.clone())),
                Arc::new(UInt64Array::from(batch.amounts.clone())),
                Arc::new(BinaryArray::from_vec(
                    batch
                        .scripts
                        .iter()
                        .map(|script| script.as_bytes())
                        .collect(),
                )),
            ];
            let rb = RecordBatch::try_new(schema.clone(), arrays)?;
            let rb = arrow::compute::take_record_batch(
                &rb,
                &arrow::compute::sort_to_indices(rb.column_by_name("script").unwrap(), None, None)?,
            )?;
            writer.write(&rb)?;
            writer.flush()?; // close row group

            batch.txids.clear();
            batch.vouts.clear();
            batch.heights.clear();
            batch.coinbases.clear();
            batch.amounts.clear();
            batch.scripts.clear();

            info!(
                "Dumped {}M UTXOs: {:.1}%",
                i / 1000000,
                (100.0 * i as f32) / (num_utxos as f32)
            );
        }
    }
    writer.close()?;
    Ok(())
}

#[derive(Default)]
struct Batch {
    txids: Vec<String>,
    vouts: Vec<u64>,
    heights: Vec<u64>,
    coinbases: Vec<bool>,
    amounts: Vec<u64>,
    scripts: Vec<ScriptBuf>,
}
