use std::ops::Index;
use std::cmp::min;

#[derive(Debug)]
pub struct MultiSlice<'g> {
    slices: Vec<&'g str>,
    total_length: usize // combined length of all slices
}

impl<'g> MultiSlice<'g> {

    pub fn new() -> Self {
        return MultiSlice {
            slices: vec!(),
            total_length: 0
        };
    }

    pub fn push(&mut self, slice: &'g str) {
        self.slices.push(slice);
        self.total_length += slice.len();
    }

    pub fn get(&self, index: usize) -> Option<&'g str> { // could be an implementation of SliceIndex, but that's nightly-only
        self.slices.get(index).and_then(|slice| Some(*slice))
    }

    fn get_next_non_empty_slice(&self, index: usize) -> Option<(usize, &'g str)> {
        for (i, string) in self.slices[index..].iter().enumerate() {
            if string.len() > 0 {
                return Some((i + index, string));
            }
        }
        return None;
    }

    pub fn get_combined_length(&self) -> usize {
        return self.total_length;
    }

    pub fn matches_string_start(&self, string: &str) -> bool {
        let mut i = 0;
        let string_len = string.len();
        for slice in &self.slices {
            let slice_len = slice.len();
            if slice_len > string_len - i || **slice != string[i..i + slice_len] {
                return false;
            }
            i += slice_len;
        }
        return true;
    }

    pub fn find_all_occurences_in<'s>(&'g self, string: &'s str) -> AllMultiSliceOccurencesIterator<'g, 's> {
        return AllMultiSliceOccurencesIterator::<'g, 's>::new(self, string);
    }
}

impl<'g> Index<usize> for MultiSlice<'g> {
    type Output = &'g str;
    fn index(&self, index: usize) -> &Self::Output {
        return &self.slices[index];
    }
}

// impl<'g, Idx> Index<Idx> for MultiSlice<'g>
// where Idx: std::slice::SliceIndex<&'g str> + std::slice::SliceIndex<[&'g str]>{
//     type Output = &'g str;
//     fn index(&self, index: Idx) -> &&str {
//         return self.slices[index];
//     }
// }

impl<'g> From<&'g str> for MultiSlice<'g> {
    fn from(slice: &'g str) -> MultiSlice<'g> {
        let mut slices = MultiSlice::new();
        slices.push(slice);
        return slices;
    }
}

// FIXME: this doesn't work for fixed-length arrays
impl<'g> From<&[&'g str]> for MultiSlice<'g> {
    fn from(slices: &[&'g str]) -> MultiSlice<'g> {
        return MultiSlice {
            slices: Vec::from(slices),
            total_length: slices.iter().map(|slice| slice.len()).sum(),
        }
    }
}

impl<'g> PartialEq<str> for MultiSlice<'g> {
    fn eq(&self, other: &str) -> bool {
        let mut position = 0;
        for slice in &self.slices {
            if **slice == other[position..position + other.len()] {
                position += other.len();
            } else {
                return false;
            }
        }
        return true
    }
}
impl<'g> Eq for MultiSlice<'g> {}

impl<'g> PartialEq<MultiSlice<'g>> for MultiSlice<'g> {
    fn eq(&self, other: &Self) -> bool {
        let mut left_slice_no = 0;
        let mut right_slice_no = 0;
        let mut left_slice_index : usize = 0;
        let mut right_slice_index : usize = 0;
        loop {
            let left_slice = self.get_next_non_empty_slice(left_slice_no);
            let right_slice = other.get_next_non_empty_slice(right_slice_no);
            if left_slice.is_none() && !right_slice.is_none() {
                return false
            } else if right_slice.is_none() && !left_slice.is_none() {
                return false
            } else if left_slice.is_none() && right_slice.is_none() {
                return true
            }
            let left_content = left_slice.expect("above if statements catch any case where left_slice is None");
            let right_content = right_slice.expect("above if statements catch any case where left_slice is None");
            left_slice_no = left_content.0;
            right_slice_no = right_content.0;
            let left_slice = left_content.1;
            let right_slice = right_content.1;
            let chars_remaining_left = left_slice.len() - left_slice_index;
            let chars_remaining_right = right_slice.len() - right_slice_index;
            let chars_to_be_compared = min(chars_remaining_left, chars_remaining_right);
            if left_slice[left_slice_index..left_slice_index + chars_to_be_compared] != right_slice[right_slice_index..right_slice_index + chars_to_be_compared] {
                return false
            } else {
                if chars_to_be_compared == chars_remaining_left {
                    left_slice_no = left_slice_no + 1;
                    left_slice_index = 0;
                } else {
                    left_slice_index += chars_to_be_compared;
                }
                if chars_to_be_compared == chars_remaining_right {
                    right_slice_no = right_slice_no + 1;
                    right_slice_index = 0;
                } else {
                    right_slice_index += chars_to_be_compared;
                }
            }
        }
    }
}

pub struct AllMultiSliceOccurencesIterator<'g, 's> {
    slices: &'g MultiSlice<'g>,
    string: &'s str,
    first_non_empty_slice: Option<&'g str>,
    next_search_position: usize,
}

impl<'g, 's> AllMultiSliceOccurencesIterator<'g, 's> {
    fn new(slices: &'g MultiSlice<'g>, string: &'s str) -> Self {
        return AllMultiSliceOccurencesIterator {
            slices: slices,
            string: string,
            first_non_empty_slice: slices.get_next_non_empty_slice(0).map(|(_, slice)| slice),
            next_search_position: 0,
        }
    }
}

impl<'g, 's> Iterator for AllMultiSliceOccurencesIterator<'g, 's> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        match self.first_non_empty_slice {
            Option::None => {
                let current_search_position = self.next_search_position;
                if current_search_position <= self.string.len() {
                    self.next_search_position += 1;
                    return Some(current_search_position);
                } else {
                    return None;
                }
            },
            Option::Some(slice) => {
                while self.next_search_position < self.string.len() {
                    let current_search_position = self.next_search_position;
                    let next_occurence = self.string[current_search_position..].find(slice);
                    match next_occurence {
                        None => {
                            self.next_search_position = self.string.len();
                            return None
                        },
                        Some(index) => {
                            let absolute_position = current_search_position + index;
                            self.next_search_position = absolute_position + 1;
                            if self.slices.matches_string_start(&self.string[absolute_position..]) {
                                return Some(absolute_position);
                            }
                        }
                    }
                }
                return None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::multislice::MultiSlice;

    #[test]
    fn test_get_empty() {
        let ms = MultiSlice::new();
        let slice = ms.get(0);
        assert!(slice.is_none());
    }

    #[test]
    fn test_get_single() {
        let ms = MultiSlice::from("abc");
        let slice = ms.get(0);
        assert_eq!(slice, Some("abc"));
    }

    #[test]
    fn test_get_two() {
        let mut ms = MultiSlice::from("abc");
        ms.push("def");
        assert_eq!(ms.get(0), Some("abc"));
        assert_eq!(ms.get(1), Some("def"));
    }

    #[test]
    fn test_equality_with_same_slice() {
        let left = MultiSlice::from("abc");
        let right = MultiSlice::from("abc");
        assert_eq!(left, right);
    }

    #[test]
    fn test_inequality_with_single_slice() {
        let left = MultiSlice::from("abc");
        let right = MultiSlice::from("def");
        assert_ne!(left, right);
    }

    #[test]
    fn test_equality_with_slice_split_into_two() {
        let left = MultiSlice::from("abcd");
        let mut right = MultiSlice::new();
        right.push("ab");
        right.push("cd");
        assert_eq!(left, right);
    }

    #[test]
    fn test_equality_with_overlapping_slices() {
        let mut left = MultiSlice::new();
        left.push("abc");
        left.push("def");
        let mut right = MultiSlice::new();
        right.push("ab");
        right.push("cd");
        right.push("ef");
        assert_eq!(left, right);
    }

    #[test]
    fn test_equality_of_empty_ms_with_empty_string() {
        let left = MultiSlice::new();
        let right = MultiSlice::from("");
        assert_eq!(left, right);
    }

    #[test]
    fn test_equality_with_trailing_empty_string() {
        let left = MultiSlice::from("42");
        let mut right = MultiSlice::from("42");
        right.push("");
        assert_eq!(left, right);
    }

    #[test]
    fn test_equality_with_preceding_empty_string() {
        let mut left = MultiSlice::new();
        left.push("");
        left.push("4");
        left.push("2");
        let right = MultiSlice::from("42");
        assert_eq!(left, right);
    }

    #[test]
    fn test_equality_with_interspersed_empty_strings() {
        let mut left = MultiSlice::new();
        left.push("Hell");
        left.push("");
        left.push("o, ");
        left.push("Worl");
        left.push("");
        left.push("d");
        left.push("");
        left.push("!");
        let right = MultiSlice::from(&[
            "He", "", "llo", "", ", W", "orl", "", "d!"
        ][..]);
        assert_eq!(left, right);
    }

    #[test]
    fn test_inequality_with_interspersed_empty_strings() {
        let mut left = MultiSlice::new();
        left.push("");
        left.push("ab");
        left.push("");
        left.push("cd");
        left.push("");
        left.push("");
        let right = MultiSlice::from(&["a", "", "", "bc", "d", "", "e", "", "f"][..]);
        left.push("Ef"); // uppercase E vs. lower-case e
        assert_ne!(left, right);
    }

    #[test]
    fn test_combined_length_empty() {
        let ms = MultiSlice::new();
        assert_eq!(ms.get_combined_length(), 0);
    }

    #[test]
    fn test_combined_length_with_empty_string() {
        let ms = MultiSlice::from("");
        assert_eq!(ms.get_combined_length(), 0);
    }

    #[test]
    fn test_combined_length_with_multiple_empty_strings() {
        let ms = MultiSlice::from(&["", ""][..]);
        assert_eq!(ms.get_combined_length(), 0);
    }

    #[test]
    fn test_combined_length_with_one_string() {
        let ms = MultiSlice::from("abc");
        assert_eq!(ms.get_combined_length(), 3);
    }

    #[test]
    fn test_combined_length_with_multiple_strings() {
        let ms = MultiSlice::from(&["abc", "de", "f"][..]);
        assert_eq!(ms.get_combined_length(), 6);
    }

    #[test]
    fn test_combined_length_with_empty_strings_interspersed() {
        let ms = MultiSlice::from(&["", "ab", "", "", "c", "defgh", "", "i", ""][..]);
        assert_eq!(ms.get_combined_length(), 9);
    }

    #[test]
    fn test_empty_multislice_matches_at_string_start() {
        let ms = MultiSlice::new();
        assert!(ms.matches_string_start("abc"));
        assert!(ms.matches_string_start(""));
        assert!(ms.matches_string_start("42"));
    }

    #[test]
    fn test_ms_with_empty_strings_matches_at_string_start() {
        let mut ms = MultiSlice::new();
        ms.push("");
        ms.push("");
        assert!(ms.matches_string_start(""));
        assert!(ms.matches_string_start("abc"));
        assert!(ms.matches_string_start("4711"));
    }

    #[test]
    fn test_ms_not_matches_empty_string() {
        let ms = MultiSlice::from(&["", "", "a"][..]);
        assert!(!ms.matches_string_start(""));
    }

    #[test]
    fn test_ms_matches_identical_string_at_start() {
        let ms = MultiSlice::from("abc");
        assert!(ms.matches_string_start("abc"));
    }

    #[test]
    fn test_split_ms_matches_identical_string_at_start() {
        let ms = MultiSlice::from(&["ab", "", "c", ""][..]);
        assert!(ms.matches_string_start("abc"));
    }

    #[test]
    fn test_split_ms_matches_longer_string_at_start() {
        let ms = MultiSlice::from(&["", "", "a", "", "", "bc"][..]);
        assert!(ms.matches_string_start("abcd"));
    }

    #[test]
    fn test_split_ms_matches_subslice_of_string() {
        let ms = MultiSlice::from(&["", "a", "", "", "n", ""][..]);
        assert!(ms.matches_string_start(&"banana"[1..]));
        assert!(ms.matches_string_start(&"banana"[3..]));
        assert!(!ms.matches_string_start(&"banana"[5..]));
    }

    #[test]
    fn test_ms_not_matches_substring_at_start() {
        let ms = MultiSlice::from("123");
        assert!(!ms.matches_string_start("12"));
    }

    #[test]
    fn test_split_ms_not_matches_independent_string() {
        let ms = MultiSlice::from(&["", "", "a", "b", "", "", "cdef", "", ""][..]);
        assert!(!ms.matches_string_start("foo"));
    }

    #[test]
    fn test_ms_not_matches_at_string_start_but_later() {
        let ms = MultiSlice::from("def");
        assert!(!ms.matches_string_start("abcdef"));
        assert!(ms.matches_string_start(&"abcdef"[3..]));
    }

    #[test]
    fn test_find_all_occurences_with_emtpy_slice_and_string() {
        let ms = MultiSlice::new();
        let occurences : Vec<usize> = ms.find_all_occurences_in("").collect();
        assert_eq!(occurences.as_slice(), &[0]);
    }

    #[test]
    fn test_find_all_occurences_with_empty_slice_and_non_empty_string() {
        let ms = MultiSlice::new();
        let occurences : Vec<usize> = ms.find_all_occurences_in("abc").collect();
        assert_eq!(occurences.as_slice(), &[0, 1, 2, 3]);
    }

    #[test]
    fn test_find_all_occurences_with_slice_of_empty_string_and_non_empty_string() {
        let ms = MultiSlice::from("");
        let occurences : Vec<usize> = ms.find_all_occurences_in("ab").collect();
        assert_eq!(occurences.as_slice(), &[0, 1, 2]);
    }

    #[test]
    fn test_find_all_occurences_with_slice_of_empty_strings_and_non_empty_string() {
        let mut ms = MultiSlice::from("");
        ms.push("");
        ms.push("");
        let occurences : Vec<usize> = ms.find_all_occurences_in("foobar").collect();
        assert_eq!(occurences.as_slice(), &[0, 1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn test_find_all_occurences_with_non_empty_slice_and_empty_string() {
        let ms = MultiSlice::from("a");
        let occurences : Vec<usize> = ms.find_all_occurences_in("").collect();
        assert_eq!(occurences.as_slice(), &[]);
    }

    #[test]
    fn test_find_all_occurences_with_non_empty_slice_and_matching_string() {
        let ms = MultiSlice::from("Hello, World");
        let occurences : Vec<usize> = ms.find_all_occurences_in("Hello, World").collect();
        assert_eq!(occurences.as_slice(), &[0]);
    }

    #[test]
    fn test_find_all_occurences_with_split_multislice_and_matching_string() {
        let mut ms = MultiSlice::from("Hello, ");
        ms.push("World");
        let occurences : Vec<usize> = ms.find_all_occurences_in("Hello, World").collect();
        assert_eq!(occurences.as_slice(), &[0]);
    }

    #[test]
    fn test_find_all_occurences_with_split_multislice_and_partial_string() {
        let mut ms = MultiSlice::from("Hello, ");
        ms.push("World");
        let occurences : Vec<usize> = ms.find_all_occurences_in("Hello, ").collect();
        assert_eq!(occurences.as_slice(), &[]);
    }

    #[test]
    fn test_find_all_occurences_with_longer_string() {
        let ms = MultiSlice::from("llo");
        let occurences : Vec<usize> = ms.find_all_occurences_in("Hello, World!").collect();
        assert_eq!(occurences.as_slice(), &[2]);
    }

    #[test]
    fn test_find_all_occurences_with_split_multislice_within_string() {
        let mut ms = MultiSlice::from("");
        ms.push("");
        ms.push("el");
        ms.push("");
        ms.push("lo");
        ms.push("");
        let occurences : Vec<usize> = ms.find_all_occurences_in("Hello, World!").collect();
        assert_eq!(occurences.as_slice(), &[1]);
    }

    #[test]
    fn test_find_all_occurences_with_multiple_occurences() {
        let mut ms = MultiSlice::from("");
        ms.push("a");
        ms.push("");
        ms.push("");
        ms.push("n");
        ms.push("");
        let occurences : Vec<usize> = ms.find_all_occurences_in("banana").collect();
        assert_eq!(occurences.as_slice(), &[1, 3]);
    }

    #[test]
    fn test_find_all_occurences_with_multiple_occurences_again() {
        let ms = MultiSlice::from(&["", "a", "", "", "n", "", ""][..]);
        let occurences : Vec<usize> = ms.find_all_occurences_in("ananas").collect();
        assert_eq!(occurences.as_slice(), &[0, 2]);
    }

}