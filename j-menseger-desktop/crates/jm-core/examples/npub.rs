fn main() {
    let p = std::env::args().nth(1).unwrap();
    let c = rusqlite::Connection::open(&p).unwrap();
    let sk: String = c.query_row("SELECT v FROM kv WHERE k='account_sk'", [], |r| r.get(0)).unwrap();
    let sk = sk.trim_matches('"');
    let k = jm_core::identity::keys_from_any(sk).unwrap();
    println!("{}", jm_core::identity::npub(&k.public_key()));
}
