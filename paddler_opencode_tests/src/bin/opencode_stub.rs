use std::env::args;
use std::env::current_dir;
use std::env::var_os;
use std::io::Result;
use std::io::Write as _;
use std::io::stdout;

fn main() -> Result<()> {
    let forwarded_arguments: Vec<String> = args().skip(1).collect();
    let mut output = stdout().lock();

    current_dir()
        .and_then(|working_directory| {
            writeln!(
                output,
                "{}\n{:?}\n{:?}\n{:?}\n{}",
                working_directory.display(),
                var_os("PWD"),
                var_os("HOME"),
                var_os("XDG_CONFIG_HOME"),
                forwarded_arguments.join(" ")
            )
        })
        .and_then(|()| output.flush())
}
