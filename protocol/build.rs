fn main() {
    println!("cargo:rerun-if-changed=protocols.xml");

    let xml = std::fs::read_to_string("protocols.xml").unwrap();
    let protocols = ascalon_protocol_schema::parse(&xml);
}
