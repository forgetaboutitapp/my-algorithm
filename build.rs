fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = prost_build::Config::new();
    config.btree_map(["."]);
    config.compile_protos(&["../protobufs/scheduler.proto"], &["../protobufs"])?;
    Ok(())
}
