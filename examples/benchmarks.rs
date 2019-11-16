use std::fs;
use std::path::PathBuf;
use std::fmt::Debug;
use std::env;
use std::str::FromStr;
use std::time::Instant;

use dary::double_array::DoubleArray;
use dary::trie::Trie;

use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use serde_derive::{Serialize, Deserialize};

fn main() {
	if let Some(n) = env::args().nth(1) {
		let bits = u32::from_str(&n).expect("error parsing argument");
		run(bits);
	} else {
		eprintln!("Usage {} <number of elements in bits>", env::args().nth(0).unwrap());
		std::process::exit(1);
	}
}

fn run(bits: u32) {
	let len = 2.0_f64.powi(bits as i32) as usize;
	println!("data len: {}", len);

	let mut keys: Vec<String> = Vec::new();
	for _ in 0..len {
		keys.push(thread_rng().sample_iter(Alphanumeric).take(30).collect::<String>());
	}

	println!("");
	println!("benchmark 1 start");
	sub_1(&keys);
	println!("");
	println!("benchmark 2 start");
	sub_2(&keys);
}


#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct MorphemeData {
    surface: String,
    cost: usize,
}

impl MorphemeData {
    fn new(surface: &str, cost: usize) -> Self {
        MorphemeData {
            surface: surface.to_string(),
            cost: cost
        }
    }
}

fn sub_1(keys: &[String]) {
	// Trie木構築
	let start = Instant::now();
	let mut trie: Trie<MorphemeData> = Trie::new();
	for (i, key) in keys.iter().enumerate() {
		trie.set(&key, MorphemeData::new(key, i));
	}
	println!("build trie: {} sec", get_duration(start));

	// DoubleArray構築
	let start = Instant::now();
	let double_array: DoubleArray<MorphemeData> = trie.to_double_array().unwrap();
	println!("build double array: {} sec", get_duration(start));

	// DoubleArrayダンプ
	let start = Instant::now();
	let mut path: PathBuf = env::current_dir().unwrap();
	path.push("benchmarks_sub_1.dic");
	let double_array = double_array.dump(path.to_str().unwrap()).unwrap();
	println!("dump double array: {} sec", get_duration(start));

	// 検索
	let start = Instant::now();
	for (i, key) in keys.iter().enumerate() {
		assert!(double_array.get(&key).unwrap().contains(&MorphemeData::new(key, i)));
	}
	println!("get all data: {} sec", get_duration(start));

	fs::remove_file(path).unwrap();
}

fn sub_2(keys: &[String]) {
	// Trie木構築
	let start = Instant::now();
	let mut trie: Trie<u32> = Trie::new();
	for (i, key) in keys.iter().enumerate() {
		trie.set(&key, i as u32);
	}
	println!("build trie: {} sec", get_duration(start));

	// DoubleArray構築
	let start = Instant::now();
	let double_array: DoubleArray<u32> = trie.to_double_array().unwrap();
	println!("build double array: {} sec", get_duration(start));

	// DoubleArrayダンプ
	let start = Instant::now();
	let mut path: PathBuf = env::current_dir().unwrap();
	path.push("benchmarks_sub_2.dic");
	let double_array = double_array.dump(path.to_str().unwrap()).unwrap();
	println!("dump double array: {} sec", get_duration(start));

	// 検索
	let start = Instant::now();
	for (i, key) in keys.iter().enumerate() {
		assert!(double_array.get(&key).unwrap().contains(&(i as u32)));
	}
	println!("get all data: {} sec", get_duration(start));

	fs::remove_file(path).unwrap();
}

fn get_duration(start: Instant) -> f64 {
	let dur = start.elapsed();
	dur.as_nanos() as f64 / 1_000_000_000.0
}