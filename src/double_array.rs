use std::fmt::Debug;
use std::slice;
use std::mem;
use std::io::prelude::*;
use std::fs::File;
use std::fs::OpenOptions;
use std::ptr;
use memmap::*;
use std::marker::PhantomData;

pub struct DoubleArrayHeader {
    base_idx  : usize,
    check_idx : usize,
    data_idx  : usize,
    base_len  : usize,
    check_len : usize,
    data_len  : usize,
}

pub struct DoubleArray<T: Debug> {
    mmap: Mmap,
    header: DoubleArrayHeader,
    phantom: PhantomData<T>,
}

impl<T: Debug> DoubleArray<T> {

    pub fn from_arrays(base_arr: Vec<u32>, check_arr: Vec<u32>, data_arr: Vec<T>) -> Result<Self, std::io::Error> {
        // base をバイト列にする
        let base_bytes: &[u8] = unsafe {
            slice::from_raw_parts(
                base_arr.as_ptr() as *const u8,
                mem::size_of::<u32>() * base_arr.len()
            )
        };
        // check をバイト列にする
        let check_bytes: &[u8] = unsafe {
            slice::from_raw_parts(
                check_arr.as_ptr() as *const u8,
                mem::size_of::<u32>() * check_arr.len()
            )
        };
        // data をバイト列にする
        let data_bytes: &[u8] = unsafe {
            slice::from_raw_parts(
                data_arr.as_ptr() as *const u8,
                mem::size_of::<T>() * data_arr.len()
            )
        };

        // headerの生成
        let header_size: usize = mem::size_of::<DoubleArrayHeader>();
        let header = DoubleArrayHeader {
            base_idx        : header_size,
            check_idx       : header_size + base_bytes.len(),
            data_idx        : header_size + base_bytes.len() + check_bytes.len(),
            base_len        : base_arr.len(),
            check_len       : check_arr.len(),
            data_len        : data_arr.len(),
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
        (&mut mmap_mut[header.data_idx..]).write_all(data_bytes)?;
        let mmap: Mmap = mmap_mut.make_read_only()?;
        Ok(DoubleArray { mmap, header, phantom: PhantomData })
    }

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

    pub fn dump(self, output_path: &str) -> Result<Self, std::io::Error> {
        let file: File = OpenOptions::new().read(true).write(true).create(true).open(output_path)?;
        file.set_len(self.mmap.len() as u64)?;
        let mut new_mmap_mut = unsafe { MmapMut::map_mut(&file)? };
        (&mut new_mmap_mut[..]).write_all(&self.mmap)?;
        new_mmap_mut.flush()?;
        Self::from_file(output_path)
    }

    fn get_arrays(&self) -> (&[u32], &[u32], &[T]) {
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
        let data_arr: &[T] = unsafe {
            slice::from_raw_parts(
                (&self.mmap)[self.header.data_idx..].as_ptr() as *const T,
                self.header.data_len
            )
        };

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
    pub fn get(&self, key: &str) -> Option<&[T]> {
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
            let data_idx = (base_arr[value_idx] >> 8) as usize;
            let data_len = (base_arr[value_idx] & 0b11111111) as usize;
            Some(&data_arr[data_idx..(data_idx + data_len)])
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
    pub fn prefix_search<'a>(&self, key: &'a str) -> Vec<(&'a str, &[T])> {
        let (base_arr, check_arr, data_arr) = self.get_arrays();
        let mut ret: Vec<(&str, &[T])> = Vec::new();
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
                let data_idx = (base_arr[value_idx] >> 8) as usize;
                let data_len = (base_arr[value_idx] & 0b11111111) as usize;
                ret.push((&key[0..(i + 1)], &data_arr[data_idx..(data_idx + data_len)]));
            }
        }
        ret
    }


    /// ダブル配列をデバッグ目的で表示するための関数
    #[allow(dead_code)]
    pub fn debug_double_array(&self, len: usize) {
        let (base_arr, check_arr, data_arr) = self.get_arrays();
        println!("size: base={}, check={}, data={}", base_arr.len(), check_arr.len(), data_arr.len());
        println!("{:-10} | {:-10} | {:-10} |", "index", "base", "check");
        println!("{:-10} | {:-10} | {:-10} |", 0, base_arr[0], check_arr[0]);
        println!("{:-10} | {:-10} | {:-10} |", 1, base_arr[1], check_arr[1]);
        for i in 2..len {
            let check = check_arr[i];
            if  check != 0 {
                if i == base_arr[check as usize] as usize {
                    let data_idx = (base_arr[i] >> 8) as usize;
                    let data_len = (base_arr[i] & 0b11111111) as usize;
                    println!(
                        "{:-10} | {:-10} | {:-10} | {:?}",
                        i,
                        base_arr[i],
                        check_arr[i],
                        &data_arr[data_idx..(data_idx + data_len)],
                        );
                } else {
                    println!(
                        "{:-10} | {:-10} | {:-10} |",
                        i,
                        base_arr[i],
                        check_arr[i],
                        );
                }
            }
        }
    }
}

/*
use std::iter::Iterator;
struct PrefixSearchIter<'a, T> {
    idx      : usize,
    base_arr : &'a [u32],
    check_arr: &'a [u32],
    data_arr : &'a [T],
}

impl<'a, T> Iterator for PrefixSearchIter<'a, T>  {
    type Item =  &'a [T];

    fn next(&mut self) -> Option<&'a [T]> {
        Some(&self.data_arr[0..1])
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_set_new() {
        let base_arr: Vec<u32> = vec![1,2,3,4,5];
        let check_arr: Vec<u32> = vec![10,20,30,40,50];
        let data_arr: Vec<u32> = vec![100,200,300,400,500];
        let double_array: DoubleArray<u32> = DoubleArray::from_arrays(base_arr, check_arr, data_arr).ok().unwrap();
        let (base_arr, check_arr, data_arr) = double_array.get_arrays();
        assert_eq!([1,2,3,4,5]          , base_arr);
        assert_eq!([10,20,30,40,50]     , check_arr);
        assert_eq!([100,200,300,400,500], data_arr);
    }
}