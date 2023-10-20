use std::collections::HashMap;

use radix_trie::{Trie, TrieCommon};

type Date = (i32, i32);

fn next_date(cur: Date) -> Date {
    let (mut current_year, mut current_month) = cur;
    current_month += 1;
    if current_month > 12 {
        current_month = 1;
        current_year += 1;
    }
    (current_year, current_month)
}

fn get_date_range(name: &str) -> Vec<Date> {
    if let Some(a) = name.strip_prefix("single-") {
        if let Some(b) = a.strip_suffix("-board-trie.postcard") {
            let parts: Vec<_> = b.split("-").collect();
            let year = parts[0].parse().unwrap();
            let month = parts[1].parse().unwrap();
            return vec![(year, month)];
        }
    }

    return vec![];

    if let Some(a) = name.strip_suffix("-board-tries.postcard") {
        if let Some(b) = a.strip_prefix("combined-") {
            let parts: Vec<_> = b.split("+").collect();
            let left_parts: Vec<_> = parts[0].split("-").collect();
            let right_parts: Vec<_> = parts[1].split("-").collect();
            let left_year: i32 = (left_parts[0]).parse().unwrap();
            let left_month: i32 = (left_parts[1]).parse().unwrap();
            let right_year = (right_parts[0]).parse().unwrap();
            let right_month = (right_parts[1]).parse().unwrap();
            let mut current_month = left_month;
            let mut current_year = left_year;
            let mut output = vec![];
            while !(current_month == right_month && current_year == right_year) {
                output.push((current_year, current_month));
                (current_year, current_month) = next_date((current_year, current_month));
            }
            output.push((current_year, current_month));

            return output;
        }
    }

    vec![]
}

/// Check if the left range ends at the time that the right one begins.
/// This makes them suitable for joining.
///
/// The ranges need to be sorted, and in the correct order,
/// for this to return true.
fn are_adjacent(left: &[Date], right: &[Date]) -> bool {
    if left.is_empty() || right.is_empty() {
        return false;
    }
    println!("{left:?} {right:?}");
    let left_end = left.last().unwrap();
    let right_begin = right.first().unwrap();
    next_date(*left_end) == *right_begin
}

#[tokio::main]
async fn main() {
    // List the files in the target directory and check if they are board tries, and for what dates.
    let mut ranges = HashMap::new();
    for file in std::fs::read_dir("../hugedata").unwrap() {
        let file_name = file.unwrap().file_name();
        let name = file_name.to_string_lossy();
        let range = get_date_range(&name);
        ranges.insert(name.to_string(), range);
    }

    for (left_name, left_range) in ranges.iter() {
        for (right_name, right_range) in ranges.iter() {
            if are_adjacent(&left_range, &right_range) {
                perform_merge(left_name, right_name, left_range, right_range).await;
                return;
            }
        }
    }

    println!("No merges possible currently");
}

async fn perform_merge(
    left_name_ref: &str,
    right_name_ref: &str,
    left_range: &[(i32, i32)],
    right_range: &[(i32, i32)],
) {
    let left_name = left_name_ref.to_string();
    let right_name = right_name_ref.to_string();

    let left_file_proc = tokio::task::spawn_blocking(move || {
        println!("Loading left file {left_name}...");
        let left_file = std::fs::OpenOptions::new()
            .read(true)
            .open(format!("../hugedata/{left_name}"))
            .unwrap();
        let reader = std::io::BufReader::new(left_file);
        let mut buf = [0; 32 * 1024];
        let left_trie: radix_trie::Trie<Vec<u8>, usize> =
            postcard::from_io((reader, &mut buf)).unwrap().0;
        println!("Loading left file completed!");
        left_trie
    });
    let right_file_proc = tokio::task::spawn_blocking(move || {
        println!("Loading right file {right_name}...");
        let right_file = std::fs::OpenOptions::new()
            .read(true)
            .open(format!("../hugedata/{right_name}"))
            .unwrap();
        let mut buf = [0; 32 * 1024];

        let reader = std::io::BufReader::new(right_file);
        let right_trie: radix_trie::Trie<Vec<u8>, usize> =
            postcard::from_io((reader, &mut buf)).unwrap().0;
        println!("Loading right file completed!");
        right_trie
    });
    let mut left_trie: Trie<Vec<u8>, usize> = left_file_proc.await.unwrap();
    let right_trie: Trie<Vec<u8>, usize> = right_file_proc.await.unwrap();

    let left_name = left_name_ref.to_string();
    let right_name = right_name_ref.to_string();

    let before = left_trie.len();
    println!("Initial sizes:");
    println!("Left: {}", before);
    println!("Right: {}", right_trie.len());
    println!("Uniques counts:");
    let left_unique_count = left_trie.values().filter(|v| **v == 0).count();
    println!("Left: {left_unique_count}");
    let right_unique_count = right_trie.values().filter(|v| **v == 0).count();
    println!("Right: {right_unique_count}");

    let total_entries = right_trie.len();
    let mut remaining_entries = right_trie.len();
    for (k, right_v) in right_trie.iter() {
        left_trie.map_with_default(k.clone(), |left_v| *left_v += right_v, *right_v);
        remaining_entries -= 1;
        if remaining_entries % 1000 == 0 {
            println!("Remaining: {remaining_entries}\t/\t{total_entries}");
        }
    }

    println!("Before: {before}");
    println!("After: {}", left_trie.len());

    let unique_count = left_trie.values().filter(|v| **v == 0).count();
    println!("New unique count: {unique_count}");

    println!("Completed merge in memory, writing to disk");
    let new_left = left_range.first().unwrap();
    let new_right = right_range.last().unwrap();

    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!(
            "../hugedata/combined-{}-{}+{}-{}-board-tries.postcard",
            new_left.0, new_left.1, new_right.0, new_right.1
        ))
        .unwrap();
    let buf = std::io::BufWriter::new(file);
    println!("New file covering {new_left:?} to {new_right:?} ready, writing...");
    postcard::to_io(&left_trie, buf).unwrap();

    println!("Write completed! deleting source files");
    std::fs::remove_file(format!("../hugedata/{left_name}")).unwrap();
    std::fs::remove_file(format!("../hugedata/{right_name}")).unwrap();
}
