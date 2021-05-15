# Running via packfile is not necessary, but here is how it works

# Runs this command
cargo run --bin cli --package cli -- pack api_triangle.pack

# Then run this:
#cargo run --example asset_triangle -- --packfile out.pack