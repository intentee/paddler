use std::env::args;
use std::io::Result;
use std::io::Write as _;
use std::io::stdout;

fn main() -> Result<()> {
    let forwarded_arguments: Vec<String> = args().skip(1).collect();
    let mut output = stdout().lock();

    writeln!(output, "{}", forwarded_arguments.join(" "))?;

    output.flush()
}
