use std::fmt::Debug;

use super::bit_cache::BitCache;
use crate::double_array::DoubleArray;

use bincode;
use serde::Serialize;
use serde::de::DeserializeOwned;

struct Node<T> {
    key   : u8,
    values: Vec<T>,
    nexts : Vec<Node<T>>,
}

pub struct Trie<T: Serialize + DeserializeOwned + Debug> {
    root: Node<T>,
    len: usize,
}

impl<T: Serialize + DeserializeOwned + Debug> Trie<T> {
    pub fn new() -> Trie<T> {
        Trie {
            root: Node { key: 0, values: Vec::new(), nexts: Vec::new() },
            len: 0,
        }
    }

    /// trieにノードを追加する
    /// 一つのkeyにつき256個までの値を登録できる
    /// 超えた場合はpanic
    ///
    /// # Arguments
    ///
    /// * `key`   - 追加するキー
    /// * `value` - キーに対応する値
    pub fn set(&mut self, key: &str, value: T) {
        let mut node = &mut self.root;
        for &k in key.as_bytes() {
            match node.nexts.binary_search_by(|probe| probe.key.cmp(&k)) {
                Ok(i) => {
                    node = &mut node.nexts[i];
                },
                Err(i) => {
                    node.nexts.insert(i, Node { key: k, values: Vec::new(), nexts: Vec::new() });
                    node = &mut node.nexts[i];
                }
            }
        }
        self.len += 1;
        node.values.push(value);
    }

    /// trieを探索する
    /// keyに対応する値が見つかったら値のスライスを返す
    ///
    /// # Arguments
    ///
    /// * `key` - 探索するkey
    pub fn get(&self, key: &str) -> Option<&[T]> {
        let mut node = &self.root;
        for &k in key.as_bytes() {
            match node.nexts.binary_search_by(|probe| probe.key.cmp(&k)) {
                Ok(i) => {
                    node = &node.nexts[i];
                },
                Err(_) => {
                    return None;
                }
            }
        }
        if node.values.is_empty() {
            None
        } else {
            Some(&node.values)
        }
    }


    /// トライ木をダブル配列に変換する
    ///
    /// # Arguments
    ///
    /// * `len` - ダブル配列の初期サイズ
    pub fn to_double_array(self) -> Result<DoubleArray<T>, std::io::Error> {
        let max_key = u8::max_value() as usize + 1;      // keyが取りうる値のパターン
        let mut len = if max_key > (4 * self.len) { max_key } else { 4 * self.len };
        let mut base_arr: Vec<u32>  = vec![0; len];
        let mut check_arr: Vec<u32> = vec![0; len];
        let mut data_arr: Vec<u8>   = Vec::with_capacity(self.len);
        let mut bit_cache: BitCache = BitCache::new();
        bit_cache.set(0);
        bit_cache.set(1);
        let mut stack: Vec<(usize, Node<T>)> = Vec::with_capacity(self.len);
        if !self.root.nexts.is_empty() {
            stack.push((1, self.root));
        }

        while !stack.is_empty() {
            let (curr_idx, mut node) = stack.pop().unwrap();
            bit_cache.update_start();

            // base値を探索・セット
            if !node.values.is_empty() {
                // valuesが存在する場合はkey=255のノードとして計算する
                node.nexts.push(Node { key: u8::max_value(), values: vec![], nexts: vec![] });
            }

            let base: usize = Self::find_base(&node.nexts, &bit_cache);
            base_arr[curr_idx] = base as u32;

            // 配列の長さが足りなければ配列を拡張
            if base + max_key >= len {
                len = len * 2;
                base_arr.resize(len, 0);
                check_arr.resize(len, 0);
            }

            // 新しいノードをダブル配列に登録
            for n in node.nexts {
                let i = base + (n.key as usize);
                bit_cache.set(i);
                check_arr[i] = curr_idx as u32;
                if n.key == u8::max_value() {
                    // valueノードの登録
                    // base には data の開始 index を格納する
                    base_arr[i]  = data_arr.len() as u32;
                    // data には末尾に values を追加する
                    let data = bincode::serialize(&node.values).unwrap();
                    data_arr.extend_from_slice(&data);
                } else {
                    // 通常ノードの登録
                    stack.push((i, n));
                }
            }
        }

        // 配列のりサイズ
        let new_len = match bit_cache.last_index_of_one() {
            None          => max_key,
            Some(new_len) => new_len + max_key,
        };
        base_arr.resize(new_len, 0);
        check_arr.resize(new_len, 0);
        DoubleArray::from_arrays(&base_arr, &check_arr, &data_arr)
    }

    /// 新しいbase値を探索するメソッド
    ///
    /// # Arguments
    ///
    /// * `nodes`     - 追加対象のノード
    /// * `bit_cache` - BitCacheのインスタンス
    /// * `with_zero` - key=0のノードも考慮してbase値を探す
    fn find_base(nodes: &[Node<T>], bit_cache: &BitCache) -> usize {
        if nodes.is_empty() {
                panic!("探索すべきノードがありません");
        }
        let first_key = nodes[0].key as usize;
        let mut offset = 0;
        'outer: loop {
            let empty_idx = bit_cache.find_empty_idx(offset);
            let new_base = empty_idx - first_key;
            if empty_idx < 256 {
                panic!("empty_idx={}, first_key={}", empty_idx, first_key);
            }
            // すべてのノードが重複せずに配置できるかをチェック
            'inner: for next in nodes {
                if bit_cache.get(new_base + next.key as usize) != 0 {
                    // 空じゃなかった場合はnew_baseを探すとこからやり直し
                    offset += 1;
                    continue 'outer;
                }
            }
            return new_base;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_1() {
        let mut trie: Trie<i32> = Trie::new();
        let s = String::from("abc");
        trie.set(&s, 0);
        trie.set(&s, 1);
        // 登録されたkeyと値が一致している
        assert_eq!(0, trie.get(&s).unwrap()[0]);
        assert_eq!(1, trie.get(&s).unwrap()[1]);
        let s = String::from("cba");
        // 登録されていないkeyはNoneを返す
        assert_eq!(None, trie.get(&s));
    }

    #[test]
    fn test_trie_2() {
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("abc");
        let s2 = String::from("abd");
        let s3 = String::from("zyx");
        let s4 = String::from("zwx");
        trie.set(&s1, 10);
        trie.set(&s2, 11);
        trie.set(&s3, 12);
        trie.set(&s4, 13);
        trie.set(&s1, 14);
        // 登録されたkeyと値が一致している
        assert_eq!(10, trie.get(&s1).unwrap()[0]);
        assert_eq!(14, trie.get(&s1).unwrap()[1]);
        assert_eq!(11, trie.get(&s2).unwrap()[0]);
        assert_eq!(12, trie.get(&s3).unwrap()[0]);
        assert_eq!(13, trie.get(&s4).unwrap()[0]);
    }

    #[test]
    fn test_trie_3() {
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("あいうえお");
        let s2 = String::from("あいえうお");
        let s3 = String::from("漢字");
        let s4 = String::from("平仮名");
        let s5 = String::from("片仮名");
        trie.set(&s1, 10);
        trie.set(&s2, 11);
        trie.set(&s3, 12);
        trie.set(&s4, 13);
        trie.set(&s5, 14);
        // 登録されたkeyと値が一致している
        assert_eq!(10, trie.get(&s1).unwrap()[0]);
        assert_eq!(11, trie.get(&s2).unwrap()[0]);
        assert_eq!(12, trie.get(&s3).unwrap()[0]);
        assert_eq!(13, trie.get(&s4).unwrap()[0]);
        assert_eq!(14, trie.get(&s5).unwrap()[0]);
    }

    #[test]
    fn test_find_base_1() {
        let nodes: Vec<Node<u32>> = vec![
            Node::<u32> { key: 2  , values: vec![], nexts: vec![] },
            Node::<u32> { key: 5  , values: vec![], nexts: vec![] },
            Node::<u32> { key: 255, values: vec![], nexts: vec![] },
        ];
        let mut bit_cache = BitCache::new();

        // 探索開始位置 = 256。空きindex = 256
        // base値 = 空きindex - 先頭ノードのkey = 256 - 2 = 254
        assert_eq!(254, Trie::find_base(&nodes, &bit_cache));

        // 0 ~ 399, 500 ~ 999 を埋める
        (256..400).for_each(|i| bit_cache.set(i));
        (500..1000).for_each(|i| bit_cache.set(i));

        // 探索開始位置 = 256。空きindex = 1000
        // base値 = 空きindex - 先頭ノードのkey = 1000 - 2 = 998
        assert_eq!(998, Trie::find_base(&nodes, &bit_cache));

        //1000..1002, 1003..1005, 1006..1255 を埋める
        (1000..1002).for_each(|i| bit_cache.set(i));
        (1003..1005).for_each(|i| bit_cache.set(i));
        (1006..1255).for_each(|i| bit_cache.set(i));

        // 探索開始位置 = 256。空きindex = 1002
        // base値 = 空きindex - 先頭ノードのkey = 1002 - 2 = 1000
        assert_eq!(1000, Trie::find_base(&nodes, &bit_cache));

        // 400 ~ 500 を埋める
        (400..500).for_each(|i| bit_cache.set(i));

        // 探索開始位置=1216。空きindex = 1255
        // base値 = 空きindex - 先頭ノードのkey = 1255 - 2 = 1253
        bit_cache.update_start();
        assert_eq!(1253, Trie::find_base(&nodes, &bit_cache));
    }

    #[test]
    #[should_panic(expected = "探索すべきノードがありません")]
    fn test_find_base_2() {
        let nodes: Vec<Node<u32>> = vec![];
        let bit_cache = BitCache::new();
        // nodesが空でwith_zero=falseの場合は、base値を求められないのでpanic
        Trie::find_base(&nodes, &bit_cache);
    }

    #[test]
    fn test_to_double_array_1() {
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("abc");
        let s2 = String::from("ac");
        let s3 = String::from("b");
        let s4 = String::from("bd");
        let s5 = String::from("bdc");
        trie.set(&s1, 1);
        trie.set(&s1, 2);
        trie.set(&s2, 3);
        trie.set(&s3, 4);
        trie.set(&s4, 5);
        trie.set(&s5, 6);
        let double_array = trie.to_double_array().ok().unwrap();
        // 登録されていて、data_arrに値が存在するkeyは対応する値を返す
        assert_eq!(vec![1, 2], double_array.get(&s1).unwrap());
        assert_eq!(vec![3],    double_array.get(&s2).unwrap());
        assert_eq!(vec![4],    double_array.get(&s3).unwrap());
        assert_eq!(vec![5],    double_array.get(&s4).unwrap());
        assert_eq!(vec![6],    double_array.get(&s5).unwrap());
        // 登録されているが、data_arrに値が存在しないkeyはNoneを返す
        assert_eq!(None, double_array.get("ab"));
    }

    #[test]
    fn test_to_double_array_2() {
        let trie: Trie<u32> = Trie::new();
        let double_array = trie.to_double_array().ok().unwrap();
        // 遷移できない場合はpanicする
        assert_eq!(None, double_array.get("abc"));
    }

    #[test]
    fn test_to_double_array_3() {
        // マルチバイト文字のテスト
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("おすしとビール");
        let s2 = String::from("お寿司とビール");
        let s3 = String::from("🍣🍺");
        trie.set(&s1, 1);
        trie.set(&s1, 2);
        trie.set(&s2, 3);
        trie.set(&s3, 4);
        let double_array = trie.to_double_array().ok().unwrap();
        // 登録されていて、data_arrに値が存在するkeyは対応する値を返す
        assert_eq!(vec![1, 2], double_array.get(&s1).unwrap());
        assert_eq!(vec![3]   , double_array.get(&s2).unwrap());
        assert_eq!(vec![4]   , double_array.get(&s3).unwrap());
        // 登録されているが、data_arrに値が存在しないkeyはNoneを返す
        assert_eq!(None, double_array.get("お寿"));
    }
}