use std::env;
use std::fs;
use std::path::PathBuf;
use std::fmt::Debug;

use dary::double_array::DoubleArray;
use dary::trie::Trie;

use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;
use serde_derive::{Serialize, Deserialize};

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


#[test]
fn double_array_1() {
	let mut keys: Vec<String> = Vec::new();
	for _ in 0..1000 {
		keys.push(thread_rng().sample_iter(Alphanumeric).take(10).collect::<String>());
	}

	let mut trie: Trie<u32> = Trie::new();
	for (i, key) in keys.iter().enumerate() {
		trie.set(&key, i as u32);
	}

	let double_array: DoubleArray<u32> = trie.to_double_array().unwrap();

	let mut path: PathBuf = env::current_dir().unwrap();
	path.push("test_double_array_1.dic");
	let double_array = double_array.dump(path.to_str().unwrap()).unwrap();

	for (i, key) in keys.iter().enumerate() {
		assert!(double_array.get(&key).unwrap().contains(&(i as u32)));
	}

	fs::remove_file(path).unwrap();
}

#[test]
fn double_array_2() {
	let mut keys: Vec<String> = Vec::new();
	for _ in 0..1000 {
		keys.push(thread_rng().sample_iter(Alphanumeric).take(10).collect::<String>());
	}

	let mut trie: Trie<String> = Trie::new();
	for key in keys.iter() {
		trie.set(&key, key.to_string());
	}

	let double_array: DoubleArray<String> = trie.to_double_array().unwrap();

	let mut path: PathBuf = env::current_dir().unwrap();
	path.push("test_double_array_2.dic");
	let double_array = double_array.dump(path.to_str().unwrap()).unwrap();

	for key in keys.iter() {
		assert!(double_array.get(&key).unwrap().contains(&key));
	}

	fs::remove_file(path).unwrap();
}

#[test]
fn double_array_3() {
	let mut keys: Vec<String> = Vec::new();
	for _ in 0..1000 {
		keys.push(thread_rng().sample_iter(Alphanumeric).take(10).collect::<String>());
	}

	let mut trie: Trie<MorphemeData> = Trie::new();
	for (i, key) in keys.iter().enumerate() {
		trie.set(&key, MorphemeData::new(key, i));
	}

	let double_array: DoubleArray<MorphemeData> = trie.to_double_array().unwrap();

	let mut path: PathBuf = env::current_dir().unwrap();
	path.push("test_double_array_3.dic");
	let double_array = double_array.dump(path.to_str().unwrap()).unwrap();

	for (i, key) in keys.iter().enumerate() {
		assert!(double_array.get(&key).unwrap().contains(&MorphemeData::new(key, i)));
	}

	fs::remove_file(path).unwrap();
}