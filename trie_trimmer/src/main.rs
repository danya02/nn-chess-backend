use radix_trie::TrieCommon;

fn main() {
    println!("!!! About to delete all board positions from tries that appear only once.");
    println!("!!! This is a destructive operation!");
    println!("Press ^C within 10 seconds to cancel...");
    std::thread::sleep(std::time::Duration::from_secs(10));

    for file in std::fs::read_dir("../hugedata").unwrap() {
        let file_name = file.unwrap().file_name();
        let name = file_name.to_string_lossy();
        if name.contains("board-trie") {
            trim_trie(&name);
        }
    }
}

fn trim_trie(name: &str) {
    println!("Trimming trie {name}");
    println!("Loading it into memory...");
    let f = std::fs::OpenOptions::new()
        .read(true)
        .open(format!("../hugedata/{name}"))
        .unwrap();
    let reader = std::io::BufReader::new(f);
    let mut buf = [0; 32 * 1024];
    let mut trie: radix_trie::Trie<Vec<u8>, usize> =
        postcard::from_io((reader, &mut buf)).unwrap().0;
    println!("Loading file completed!");

    let before = trie.len();
    println!("Length before: {before}");
    println!("Removing all unique nodes...");
    let mut more_keys: bool = true;
    while more_keys {
        more_keys = false;
        let mut keys_to_delete = vec![];
        for (k, v) in trie.iter() {
            if *v == 0 {
                keys_to_delete.push(k.clone());
            }
            if keys_to_delete.len() > 8 * 1024 {
                more_keys = true;
                break;
            }
        }
        for key in keys_to_delete.drain(0..) {
            trie.remove(&key);
        }
        println!("Now remaining: {}", trie.len());
    }

    println!("Trimming complete");
    println!("Length before: \t{before}");
    println!("Length now: \t{}", trie.len());
    println!("Now overwriting {name}: DO NOT CLOSE PROGRAM NOW...");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(format!("../hugedata/{name}",))
        .unwrap();
    let buf = std::io::BufWriter::new(file);
    postcard::to_io(&trie, buf).unwrap();
    println!("Written!");
}
