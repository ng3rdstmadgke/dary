use dary::double_array::DoubleArray;
use dary::trie::Trie;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

#[test]
fn double_array() {
	let mut keys: Vec<String> = Vec::new();
	for _ in 0..1000 {
		keys.push(thread_rng().sample_iter(Alphanumeric).take(10).collect::<String>());
	}

	let mut trie: Trie<u32> = Trie::new();
	for (i, key) in keys.iter().enumerate() {
		trie.set(&key, i as u32);
	}

	let double_array: DoubleArray<u32> = trie.to_double_array().ok().unwrap();

	for (i, key) in keys.iter().enumerate() {
		assert!(double_array.get(&key).unwrap().contains(&(i as u32)));
	}
}