# dary
[![Build Status](https://travis-ci.com/ng3rdstmadgke/dary.svg?branch=master)](https://travis-ci.com/ng3rdstmadgke/dary)

# Benchmark
トライ木の構築、ダブル配列の構築、ダブル配列の検索などのベンチマークを行います。
benchmark 1 では `MorphemeData { surface: String, cost usize }` をデータとして登録したときのベンチマークを行います。
benchmark 2 では `u32` をデータとして登録したときのベンチマークを行います。

```bash
# [usage]
#   cargo run --release --example benchmarks -- <DATA_LENGTH_IN_BITS>
# 
#   <DATA_LENGTH_IN_BITS> テストするデータ数をビットで指定します。10とするとデータ数は 1024 個です。

cargo run --release --example benchmarks -- 23
```
```
    Finished release [optimized] target(s) in 0.03s
     Running `target/release/examples/benchmarks 23`
data len: 8388608

benchmark 1 start
build trie: 14.3049744 sec
build double array: 12.821794241 sec
dump double array: 2.061486205 sec
get all data: 6.9113534 sec

benchmark 2 start
build trie: 19.967433678 sec
build double array: 20.440614366 sec
dump double array: 1.854162494 sec
get all data: 5.642468931 sec
```
