//! Generate headers for the mars-bluetooth-hci library.

fn main() -> std::io::Result<()> {
    #[cfg(feature = "headers")]
    {
        use std::env;

        let args: Vec<String> = env::args().collect();
        if args.is_empty() || args.len() < 2 {
            eprintln!("Usage: {} <output_header_path>", args[0]);
            std::process::exit(1);
        }
        mars_bluetooth_hci::generate_headers(&args[1])?;
    }

    Ok(())
}
