[![Build Status](https://travis-ci.com/ng3rdstmadgke/dary.svg?branch=master)](https://travis-ci.com/ng3rdstmadgke/dary)
# dary

## Description
daryはダブル配列の構築及び、検索を実装したライブラリです。

## Benchmark
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

## Getting started 
```rust
use std::fmt::Debug;
use dary::DoubleArray;
use dary::Trie;
use serde_derive::{Serialize, Deserialize};

fn main() {
  let key1 = String::from("foo");
  let key2 = String::from("bar");
  let key3 = String::from("baz");

  let sample1 = Sample { surface: key1.clone(), cost: 1 };
  let sample2 = Sample { surface: key1.clone(), cost: 2 };
  let sample3 = Sample { surface: key2.clone(), cost: 1 };
  let sample4 = Sample { surface: key3.clone(), cost: 1 };

  let mut trie: Trie<Sample> = Trie::new();
  trie.set(&key1, sample1.clone());
  trie.set(&key1, sample2.clone());
  trie.set(&key2, sample3.clone());
  trie.set(&key3, sample4.clone());

  let double_array = trie.to_double_array().ok().unwrap();
  assert_eq!(vec![sample1, sample2], double_array.get(&key1).unwrap());
  assert_eq!(vec![sample3]         , double_array.get(&key2).unwrap());
  assert_eq!(vec![sample4]         , double_array.get(&key3).unwrap());
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Sample {
    surface: String,
    cost: usize,
}
```
