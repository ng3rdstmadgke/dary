use std::fmt::Debug;
use std::slice;
use std::mem;
use std::io::prelude::*;
use std::fs::File;
use std::fs::OpenOptions;
use std::ptr;
use std::marker::PhantomData;

use crate::utils::*;

use memmap::*;
use bincode;
use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Debug)]
struct DoubleArrayHeader {
    base_idx  : usize,
    check_idx : usize,
    data_idx  : usize,
    base_len  : usize,
    check_len : usize,
}

#[derive(Debug)]
pub struct DoubleArray<T: Serialize + DeserializeOwned + Debug> {
    mmap: Mmap,
    header: DoubleArrayHeader,
    phantom: PhantomData<T>,
}

impl<T: Serialize + DeserializeOwned + Debug> DoubleArray<T> {

    /// base配列, check配列, data配列からDoubleArrayインスタンスを生成する。
    ///
    /// # Arguments
    ///
    /// * `base_arr`   - base配列
    /// * `check_arr`  - check配列
    /// * `data_bytes` - data配列
    pub fn from_arrays(base_arr: &[u32], check_arr: &[u32], data_bytes: &[u8]) -> Result<Self, std::io::Error> {
        let base_bytes = to_bytes(base_arr);
        let check_bytes = to_bytes(check_arr);
        // headerの生成
        let header_size: usize = mem::size_of::<DoubleArrayHeader>();
        let header = DoubleArrayHeader {
            base_idx        : header_size,
            check_idx       : header_size + base_bytes.len(),
            data_idx        : header_size + base_bytes.len() + check_bytes.len(),
            base_len        : base_arr.len(),
            check_len       : check_arr.len(),
        };

        // header をバイト列にする
        let header_bytes: &[u8] = unsafe {
            slice::from_raw_parts(
                &header as *const DoubleArrayHeader as *const u8,
                header_size,
            )
        };

        let bytes_len = header_size + base_bytes.len() + check_bytes.len() + data_bytes.len();
        let mut mmap_options = MmapOptions::new();
        let mut mmap_mut: MmapMut = mmap_options.len(bytes_len).map_anon()?;
        (&mut mmap_mut[..]).write_all(header_bytes)?;
        (&mut mmap_mut[header.base_idx..]).write_all(base_bytes)?;
        (&mut mmap_mut[header.check_idx..]).write_all(check_bytes)?;
        (&mut mmap_mut[header.data_idx..]).write_all(&data_bytes)?;
        let mmap: Mmap = mmap_mut.make_read_only()?;
        Ok(DoubleArray { mmap, header, phantom: PhantomData })
    }

    /// u8の配列からDoubleArrayインスタンスを生成する。
    ///
    /// # Arguments
    ///
    /// * `bytes` - base配列, check配列, data配列を u8 の配列として連結させた配列
    pub fn from_slice(bytes: &[u8]) -> Result<Self, std::io::Error> {
        let mut mmap_options = MmapOptions::new();
        let mut mmap_mut: MmapMut = mmap_options.len(bytes.len()).map_anon()?;
        mmap_mut.copy_from_slice(bytes);
        let mmap: Mmap = mmap_mut.make_read_only()?;
        let header: DoubleArrayHeader = unsafe {
            ptr::read((&mmap).as_ptr() as *const DoubleArrayHeader)
        };
        Ok(DoubleArray { mmap, header, phantom: PhantomData })
    }

    /// ファイルからDoubleArrayインスタンスを生成する。
    ///
    /// # Arguments
    ///
    /// * `dictionary_path` - 辞書ファイルパス
    pub fn from_file(dictionary_path: &str) -> Result<Self, std::io::Error> {
        let file: File = File::open(dictionary_path)?;
        let mmap: Mmap = unsafe {
            MmapOptions::new().map(&file)?
        };
        let header: DoubleArrayHeader = unsafe {
            ptr::read((&mmap).as_ptr() as *const DoubleArrayHeader)
        };
        Ok(DoubleArray { mmap, header, phantom: PhantomData })
    }

    /// DoubleArrayをファイルにダンプする
    ///
    /// # Arguments
    ///
    /// * `output_path` - 辞書ファイルパス
    pub fn dump(self, output_path: &str) -> Result<Self, std::io::Error> {
        let file: File = OpenOptions::new().read(true).write(true).create(true).open(output_path)?;
        file.set_len(self.mmap.len() as u64)?;
        let mut new_mmap_mut = unsafe { MmapMut::map_mut(&file)? };
        (&mut new_mmap_mut[..]).write_all(&self.mmap)?;
        new_mmap_mut.flush()?;
        Self::from_file(output_path)
    }

    /// mmapをパースして、base配列, check配列, data配列 を返す。
    ///
    /// # Arguments
    ///
    /// * `output_path` - 辞書ファイルパス
    fn get_arrays(&self) -> (&[u32], &[u32], &[u8]) {
        // base_arr
        let base_arr: &[u32] = unsafe {
            slice::from_raw_parts(
                (&self.mmap)[self.header.base_idx..].as_ptr() as *const u32,
                self.header.base_len
            )
        };

        // check_arr
        let check_arr: &[u32] = unsafe {
            slice::from_raw_parts(
                (&self.mmap)[self.header.check_idx..].as_ptr() as *const u32,
                self.header.check_len
            )
        };

        // data_arr
        let data_arr: &[u8] = &self.mmap[self.header.data_idx..];

        (base_arr, check_arr, data_arr)
    }

    /// ダブル配列から指定されたkeyを探索する関数
    /// 途中で遷移できなくなった場合、data_arrに値が存在しない場合はNoneを返す
    /// 遷移ができて、data_arrに値が存在する場合はdata_arrのスライスを返す
    /// デバッグ用
    ///
    /// # Arguments
    ///
    /// * `key`       - 探索対象の文字列
    pub fn get(&self, key: &str) -> Option<Vec<T>> {
        let (base_arr, check_arr, data_arr) = self.get_arrays();

        let mut idx  = 1;
        let mut base = base_arr[idx] as usize;

        for &byte in key.as_bytes() {
            let next_idx = base + (byte as usize);
            if  check_arr[next_idx] as usize != idx {
                return None;
            }
            idx  = next_idx;
            base = base_arr[idx] as usize;
        }
        let value_idx = base + (u8::max_value() as usize);
        if check_arr[value_idx] as usize == idx {
            let data_idx = base_arr[value_idx] as usize;
            let data: Vec<T> = bincode::deserialize(&data_arr[data_idx..]).unwrap();
            Some(data)
        } else {
            None
        }
    }

    /// ダブル配列で共通接頭辞検索を行う
    /// デバッグ用
    ///
    /// # Arguments
    ///
    /// * `key`       - 探索対象の文字列
    pub fn prefix_search<'a>(&self, key: &'a str) -> Vec<(&'a str, Vec<T>)> {
        let (base_arr, check_arr, data_arr) = self.get_arrays();
        let mut ret: Vec<(&str, Vec<T>)> = Vec::new();
        let mut idx = 1;
        let mut base = base_arr[idx] as usize;

        for (i, &byte) in key.as_bytes().iter().enumerate() {
            // 次のノードに遷移
            let next_idx = base + (byte as usize);
            if check_arr[next_idx] as usize != idx {
                break;
            }
            idx = next_idx;
            base = base_arr[idx] as usize;
            // value があれば戻り値の配列に追加
            let value_idx = base + (u8::max_value() as usize);
            if check_arr[value_idx] as usize == idx {
                let data_idx = base_arr[value_idx] as usize;
                let data: Vec<T> = bincode::deserialize(&data_arr[data_idx..]).unwrap();
                ret.push((&key[0..(i + 1)], data));
            }
        }
        ret
    }

    pub fn prefix_search_iter<'a>(&'a self, key: &'a str) -> PrefixSearchIter<'a, T> {
        let (base_arr, check_arr, data_arr) = self.get_arrays();
        PrefixSearchIter {
            key_ptr: 0,
            key: key,
            arr_ptr: 1,
            base_arr: base_arr,
            check_arr: check_arr,
            data_arr: data_arr,
            phantom: PhantomData,
        }
    }


    /// ダブル配列をデバッグ目的で表示するための関数
    #[allow(dead_code)]
    fn debug_double_array(&self, mut len: usize) {
        let (base_arr, check_arr, data_arr) = self.get_arrays();
        println!("size: base={}, check={}, data={}", base_arr.len(), check_arr.len(), data_arr.len());
        println!("{:-10} | {:-10} | {:-10} | {:-10}", "index", "base", "check", "data");
        println!("{:-10} | {:-10} | {:-10} |", 0, base_arr[0], check_arr[0]);
        println!("{:-10} | {:-10} | {:-10} |", 1, base_arr[1], check_arr[1]);

        len = if len < base_arr.len() { len } else { base_arr.len() };
        for i in 2..len {
            let check = check_arr[i] as usize;
            let base  = base_arr[i] as usize;
            if  check != 0 {
                if (base_arr[check] as usize) + (u8::max_value() as usize) == i {
                    // 遷移前のbase値と255を足した値が現在のインデックスと等しいとき、dataが存在する
                    let data: Vec<T> = bincode::deserialize(&data_arr[base..]).unwrap();
                    println!( "{:-10} | {:-10} | {:-10} | {:?}", i, base, check, data);
                } else {
                    println!( "{:-10} | {:-10} | {:-10} |", i, base, check);
                }
            }
        }
    }
}

use std::iter::Iterator;
pub struct PrefixSearchIter<'a, T>
    where T: Serialize + DeserializeOwned + Debug,
{
    key_ptr  : usize,
    key      : &'a str,
    arr_ptr  : usize,
    base_arr : &'a [u32],
    check_arr: &'a [u32],
    data_arr : &'a [u8],
    phantom: PhantomData<T>,
}

impl<'a, T> Iterator for PrefixSearchIter<'a, T>
    where T: Serialize + DeserializeOwned + Debug,
{
    type Item =  (&'a str, Vec<T>);

    fn next(&mut self) -> Option<(&'a str, Vec<T>)> {
        let mut base = self.base_arr[self.arr_ptr] as usize;

        while self.key_ptr < self.key.len() {
            let next_arr_ptr = base + (self.key.as_bytes()[self.key_ptr] as usize);
            self.key_ptr += 1;
            if self.check_arr[next_arr_ptr] as usize != self.arr_ptr {
                return None;
            }
            self.arr_ptr = next_arr_ptr;
            base = self.base_arr[self.arr_ptr] as usize;

            let value_idx = base + (u8::max_value() as usize);
            if self.check_arr[value_idx] as usize == self.arr_ptr {
                let data_idx = self.base_arr[value_idx] as usize;
                let data: Vec<T> = bincode::deserialize(&self.data_arr[data_idx..]).unwrap();
                return Some((&self.key[0..self.key_ptr], data));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trie::Trie;
    use std::fmt::Debug;
    use serde_derive::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MorphemeData {
        surface: String,
        cost: i32,
    }

    impl MorphemeData {
        fn new(surface: &str, cost: i32) -> Self {
            MorphemeData {
                surface: surface.to_string(),
                cost: cost
            }
        }
    }

    #[test]
    fn test_dictionary_set_new() {
        let base_arr: Vec<u32> = vec![1,2,3,4,5];
        let check_arr: Vec<u32> = vec![10,20,30,40,50];
        let data_arr: Vec<u8> = vec![100,110,120,130,140];
        let double_array: DoubleArray<u32> = DoubleArray::from_arrays(&base_arr, &check_arr, &data_arr).ok().unwrap();
        let (base_arr, check_arr, data_arr) = double_array.get_arrays();
        assert_eq!([1,2,3,4,5]          , base_arr);
        assert_eq!([10,20,30,40,50]     , check_arr);
        assert_eq!([100,110,120,130,140], data_arr);
    }

    #[test]
    fn test_get_1() {
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
        // debug_double_array(&base_arr, &check_arr, &data_arr);
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
    fn test_get_2() {
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("合沢");
        let s2 = String::from("会沢");
        let s3 = String::from("哀澤");
        let s4 = String::from("愛沢");
        let s5 = String::from("會澤");
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
        assert_eq!(None, double_array.get("合い"));
    }

    #[test]
    fn test_get_3() {
        let mut trie: Trie<MorphemeData> = Trie::new();
        let s1 = String::from("合沢");
        let s2 = String::from("会沢");
        let s3 = String::from("哀澤");
        let s4 = String::from("愛沢");
        let s5 = String::from("會澤");
        trie.set(&s1, MorphemeData::new("合沢", 1));
        trie.set(&s1, MorphemeData::new("合沢", 2));
        trie.set(&s2, MorphemeData::new("会沢", 3));
        trie.set(&s3, MorphemeData::new("哀澤", 4));
        trie.set(&s4, MorphemeData::new("愛沢", 5));
        trie.set(&s5, MorphemeData::new("會澤", 6));
        let double_array = trie.to_double_array().ok().unwrap();
        // 登録されていて、data_arrに値が存在するkeyは対応する値を返す
        assert_eq!(vec![MorphemeData::new("合沢", 1), MorphemeData::new("合沢", 2)], double_array.get(&s1).unwrap());
        assert_eq!(vec![MorphemeData::new("会沢", 3)], double_array.get(&s2).unwrap());
        assert_eq!(vec![MorphemeData::new("哀澤", 4)], double_array.get(&s3).unwrap());
        assert_eq!(vec![MorphemeData::new("愛沢", 5)], double_array.get(&s4).unwrap());
        assert_eq!(vec![MorphemeData::new("會澤", 6)], double_array.get(&s5).unwrap());
        // 登録されているが、data_arrに値が存在しないkeyはNoneを返す
        assert_eq!(None, double_array.get("合い"));
    }

    #[test]
    fn test_prefix_search_1() {
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("鳴ら");
        let s2 = String::from("鳴らしゃ");
        let s3 = String::from("鳴らし初め");
        let s4 = String::from("鳴らし初めよ");
        trie.set(&s1, 1);
        trie.set(&s1, 2);
        trie.set(&s2, 3);
        trie.set(&s3, 4);
        trie.set(&s4, 5);
        let double_array = trie.to_double_array().ok().unwrap();
        let key = String::from("鳴らし初めよ");
        let result = double_array.prefix_search(&key);
        assert_eq!(("鳴ら"       , vec![1, 2]), result[0]);
        assert_eq!(("鳴らし初め"  , vec![4]) , result[1]);
        assert_eq!(("鳴らし初めよ", vec![5]) , result[2]);
    }

    #[test]
    fn test_prefix_search_2() {
        let mut trie: Trie<u32> = Trie::new();
        let s1 = String::from("鳴ら");
        let s2 = String::from("鳴らしゃ");
        let s3 = String::from("鳴らし初め");
        let s4 = String::from("鳴らし初めよ");
        trie.set(&s1, 1);
        trie.set(&s1, 2);
        trie.set(&s2, 3);
        trie.set(&s3, 4);
        trie.set(&s4, 5);
        let double_array = trie.to_double_array().ok().unwrap();
        // double_array.debug_double_array(555);
        let key = String::from("鳴らし初めよ");
        let result: Vec<(&str, Vec<u32>)> = double_array.prefix_search_iter(&key).collect();
        assert_eq!(("鳴ら"       , vec![1, 2]), result[0]);
        assert_eq!(("鳴らし初め"  , vec![4]) , result[1]);
        assert_eq!(("鳴らし初めよ", vec![5]) , result[2]);
    }
}
