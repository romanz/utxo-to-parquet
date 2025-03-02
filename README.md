# Example

## Dump UTXO

```
$ time bitcoin-cli dumptxoutset ~/tmp/mainnet-utxo.dump latest
{      
  "coins_written": 177539910,
  "base_hash": "00000000000000000000f008cdb56953ee339f50d032935b7177921c16576ec0",
  "base_height": 886001,
  "path": "/home/user/tmp/mainnet-latest-utxo.dump",
  "txoutset_hash": "ec45d807096e1e2e211346750be00291b8252928583ce6ac234e02e27ec1fb7f",
  "nchaintx": 1161476676
}

real	3m14.533s
user	0m0.001s
sys	0m0.003s
```

## Convert to Parquet

```
$ RUST_LOG=info time cargo run --release -- -i ~/tmp/mainnet-latest-utxo.dump -o ~/tmp/mainnet-886001-utxo.parquet
   Compiling utxo-to-parquet v0.1.0 (/home/user/src/utxo-to-parquet)
    Finished `release` profile [optimized] target(s) in 2.25s
     Running `target/release/utxo-to-parquet -i /home/user/tmp/mainnet-latest-utxo.dump -o /home/user/tmp/mainnet-886001-utxo.parquet`
[2025-03-02T11:33:38.036008Z INFO  utxo_to_parquet] Bitcoin UTXO snapshot at block hash 00000000000000000000f008cdb56953ee339f50d032935b7177921c16576ec0, contains 177539910 coins, version 2
[2025-03-02T11:33:57.777568Z INFO  utxo_to_parquet] Dumped 10M UTXOs: 5.6%
[2025-03-02T11:34:17.824072Z INFO  utxo_to_parquet] Dumped 20M UTXOs: 11.3%
[2025-03-02T11:34:37.325036Z INFO  utxo_to_parquet] Dumped 30M UTXOs: 16.9%
[2025-03-02T11:34:56.710756Z INFO  utxo_to_parquet] Dumped 40M UTXOs: 22.5%
[2025-03-02T11:35:15.827649Z INFO  utxo_to_parquet] Dumped 50M UTXOs: 28.2%
[2025-03-02T11:35:35.043407Z INFO  utxo_to_parquet] Dumped 60M UTXOs: 33.8%
[2025-03-02T11:35:54.400982Z INFO  utxo_to_parquet] Dumped 70M UTXOs: 39.4%
[2025-03-02T11:36:14.379360Z INFO  utxo_to_parquet] Dumped 80M UTXOs: 45.1%
[2025-03-02T11:36:34.324211Z INFO  utxo_to_parquet] Dumped 90M UTXOs: 50.7%
[2025-03-02T11:36:53.987228Z INFO  utxo_to_parquet] Dumped 100M UTXOs: 56.3%
[2025-03-02T11:37:13.686123Z INFO  utxo_to_parquet] Dumped 110M UTXOs: 62.0%
[2025-03-02T11:37:33.171604Z INFO  utxo_to_parquet] Dumped 120M UTXOs: 67.6%
[2025-03-02T11:37:52.776147Z INFO  utxo_to_parquet] Dumped 130M UTXOs: 73.2%
[2025-03-02T11:38:12.344477Z INFO  utxo_to_parquet] Dumped 140M UTXOs: 78.9%
[2025-03-02T11:38:31.869846Z INFO  utxo_to_parquet] Dumped 150M UTXOs: 84.5%
[2025-03-02T11:38:51.309366Z INFO  utxo_to_parquet] Dumped 160M UTXOs: 90.1%
[2025-03-02T11:39:11.019227Z INFO  utxo_to_parquet] Dumped 170M UTXOs: 95.8%
[2025-03-02T11:39:25.110758Z INFO  utxo_to_parquet] Dumped 177M UTXOs: 100.0%
311.88user 42.21system 5:50.56elapsed 101%CPU (0avgtext+0avgdata 4398124maxresident)k
```

## Run SQL query

For example, list https://mempool.space/address/1BitcoinEaterAddressDontSendf59kuE unspent coins:
```sql
D SELECT txid, vout, amount, height FROM '/home/user/tmp/mainnet-886001-utxo.parquet'
  WHERE script = from_hex('76a914759d6677091e973b9e9d99f19c68fbf43e3f05f988ac')
  ORDER BY height;
┌──────────────────────────────────────────────────────────────────┬────────┬──────────┬────────┐
│                               txid                               │  vout  │  amount  │ height │
│                             varchar                              │ uint64 │  uint64  │ uint64 │
├──────────────────────────────────────────────────────────────────┼────────┼──────────┼────────┤
│ 369d241af595fc253479abe394e2f21fda05820a0416942f63266dd793035cf1 │      0 │  1000000 │ 132184 │
│ 456b21f80179f59f519ff170afd390a4474610f4a9de7368fb3a778a7f84939f │      1 │   100000 │ 132187 │
│ 519f6c9581ce27e0a59f5f8e427b672087e1f2eb1aead0d66288de62ed3e9647 │      0 │  1000000 │ 132423 │
│ d8f8b0dd7df353098f37c5b12ac070d045b61747d4283da13ac1f005ecb3b610 │      1 │    10000 │ 133402 │
│ f06c8a4af6483b1fd75c3b631af019432e3da65a5183bbc9e9bb12d703066037 │      1 │ 13370000 │ 159532 │
│ 3415c300372b0c9ecd5b6e173dee70317b1a89d8187e4e59f7e5e557f2df5704 │      1 │   420000 │ 183769 │
│ f17e24d731c4d2fbd8473f2396c2abd4b02d8157274bc3266b20bb8ad9a9985f │      1 │        1 │ 200961 │
│ 0179a4bd3c90cde8aa43bbbbc05ffd00494fe06cae0d5b60af09e9eb429c22ae │      0 │   100005 │ 204034 │
│ 544bf3a2556d2e4dc8a6cd462e01ae71bd658ded9ff493accb0cdf47b9bd45bd │      0 │        1 │ 212714 │
│ 66ceafad27e70c18b0f1e3ce2097cc7502eb736cb6ffb9d2de0b58e7a4c492f8 │      2 │ 10000000 │ 213202 │
│ 7696cec2323916d9f0cf2da2ded073ac9c25ffa961322b5fb81c0ae3f0081486 │      0 │        1 │ 213534 │
│ c07ab5d66129591674c93bb5639a7ec22a091c61e91798e34a8f0240b188775b │      1 │        1 │ 216771 │
│ af593bb96ab3398f3fb624a29f1332ed833cc3dfb8e4e4752f6d1722cbf3e2c0 │      1 │        1 │ 216773 │
│ 9ef8b279dc49a1667bd59c7306ad84e613d196253e564edb01fb3bb404fa557e │      1 │        1 │ 216802 │
│ af35735bcda63b1b4801e69c1eedec3e85de817f0b093fad1b6a38d1b12842c1 │     28 │    16000 │ 216846 │
│ 3a7b643078f892be661f4c336e2bc8f52209c32aeebea0b7c9a1ffdd2102e8c1 │      1 │   575000 │ 217464 │
│ 87ca151f3c7921e81d325464448c27c77cb93b8bd010e84f37b289879e885486 │      1 │  7000000 │ 217595 │
│ 2b8659f9d302531b77d163b538a233df40429a6c4b183cfcb0694325814cc947 │      1 │   100000 │ 217725 │
│ 2d72c25c32ba7b8a33fd88c14bbcff5905f2be8737bb5c0200931af7eef3d7b1 │      1 │   603989 │ 218332 │
│ 8a14ed8fa3a92af3ba48bf10da5b6e1f766ca89b93db3470b17be5aa9986e987 │      3 │   402784 │ 218614 │
│                                ·                                 │      · │       ·  │    ·   │
│                                ·                                 │      · │       ·  │    ·   │
│                                ·                                 │      · │       ·  │    ·   │
│ fd637c355aa68111ee712750832e175e7d9459ecac1a9a2c3ffba267e751ae5a │      1 │      546 │ 885584 │
│ eb58ac94157956c464509f099a20bbce644076fbdcb95d8184515cd84bb634f1 │      1 │      546 │ 885606 │
│ 0ab446aecffbfe6fa773f4c5ad27d674b10e0491e287f1f457985597e35460d3 │      1 │      546 │ 885659 │
│ c4877b7ace1ab03f5fe11daaad1c00bd2ca883a90c79c33032757c8276d2fa94 │      1 │      546 │ 885662 │
│ 38371bc0df26d2869579e3007e7c4f8aa387c8b495d568a6b75e1a57df82b1b0 │      0 │      546 │ 885694 │
│ dc98b46347af0e27bc6dec35f5858b67bdf7c80616481eee730bc9f61f13e2d8 │      1 │      546 │ 885711 │
│ 66a13ca373127fbc99d4336a562c41eb28fb88b94b990da70ac5a65bc1207cdd │      1 │      546 │ 885718 │
│ b645c043e00e7dee51f8a42408c88fcfce2a83b18a3433d23605e9037abd0c74 │      1 │      546 │ 885729 │
│ 65b4cc06a1252d9402239d38a58352bfb45edcfcab95fa99dc45e78820d53822 │      1 │      546 │ 885761 │
│ cfddf7ed105bd85b89f0bdedd922ce208b51bbf9b370e830e5256bc511d91d1b │      1 │      546 │ 885761 │
│ 340276f6a5145193000633663571daccf6dbb00806488a9a281a662828682117 │      0 │      546 │ 885762 │
│ 6f0102d82f8beb02a52b7578c104364e3cfa104bef623efd3697fcee9a6c2189 │      1 │      546 │ 885768 │
│ e09abb1d37be0ce298080fdd7b90a26a41f0765f18c03f41b619409a9b8976ab │      1 │      546 │ 885768 │
│ fd832e0a514ce840c6897ff9b36bf5be1140dcca52b4780dcdae8f77674c1f7e │      0 │      546 │ 885774 │
│ 47010b5f795cc4a85ef49e8330cd1efa9f3bd6f3e7bccab9b1f1ef3866059241 │      1 │      546 │ 885848 │
│ 1ad594553f47da5520c525fd48d3c3cb399c54940f356e922e9435208c9e6b9b │      1 │      546 │ 885858 │
│ a4d4be0cc051858680d92d9d3b5ce9ef6b9c3a4682e5dc58c6f96f7a9d1a3978 │      1 │      546 │ 885883 │
│ e64b98b11746451d2d417e5e16f17973c1b129d5e5727d45172245e0b296b4c7 │      1 │      546 │ 885889 │
│ 711ffa32a28e4da02cf78e4b0a729ecca058c9cd805856d889637fb3573af0d5 │      1 │      546 │ 885993 │
│ 2b317aff4141cd37340bc36ec9fa2e79385f76efa95a95e4798961e6f315d32e │      1 │      546 │ 886001 │
├──────────────────────────────────────────────────────────────────┴────────┴──────────┴────────┤
│ 4407 rows (40 shown)                                                                4 columns │
└───────────────────────────────────────────────────────────────────────────────────────────────┘
Run Time (s): real 0.632 user 5.952068 sys 1.037173
```
