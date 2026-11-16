use std::io::Read;

// `elkr layout graph.json` reads an ELK JSON graph, lays it out, and
// writes the same graph with positions and edge sections on stdout
// (or the file given by -o). The command is the simplest stable surface
// for consuming elkr from scripts and tests.
fn main() {
    let mut args = std::env::args().skip(1);
    let mut output: Option<String> = None;
    let mut input: Option<String> = None;
    let mut command = String::from("layout");

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => output = args.next(),
            "-V" | "--version" => {
                println!("elkr {}", elkr::VERSION);
                return;
            }
            "-h" | "--help" => {
                print_help();
                return;
            }
            "layout" | "dim" if input.is_none() && output.is_none() => {
                command = arg.to_string();
            }
            other => {
                if input.is_none() {
                    input = Some(other.to_string());
                } else {
                    eprintln!("elkr: unexpected argument {other}");
                    std::process::exit(2);
                }
            }
        }
    }

    let source = match input.as_deref() {
        Some("-") | None => {
            let mut buf = String::new();
            if std::io::stdin().read_to_string(&mut buf).is_err() {
                eprintln!("elkr: cannot read stdin");
                std::process::exit(1);
            }
            buf
        }
        Some(path) => match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("elkr: cannot read {path}: {e}");
                std::process::exit(1);
            }
        },
    };

    let out = if command == "dim" {
        match elkr::dim::layout_document(&source) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("elkr: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let mut graph: elkr::ElkGraph = match serde_json::from_str(&source) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("elkr: invalid ELK JSON: {e}");
                std::process::exit(1);
            }
        };
        if let Err(e) = elkr::apply(&mut graph) {
            eprintln!("elkr: {e}");
            std::process::exit(1);
        }
        match serde_json::to_string_pretty(&graph) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("elkr: cannot serialize: {e}");
                std::process::exit(1);
            }
        }
    };

    match output.as_deref() {
        None | Some("-") => println!("{out}"),
        Some(path) => {
            if let Err(e) = std::fs::write(path, out + "\n") {
                eprintln!("elkr: cannot write {path}: {e}");
                std::process::exit(1);
            }
        }
    }
}

fn print_help() {
    println!(
        "elkr {} — deterministic graph layout (ELK algorithms)

USAGE:
    elkr layout [INPUT] [-o OUTPUT]
    elkr dim [INPUT] [-o OUTPUT]

    INPUT is an ELK JSON graph file, or '-' (default) for stdin.
    OUTPUT is the laid-out graph file, or '-' (default) for stdout.

OPTIONS:
    -o, --output PATH   write to PATH instead of stdout
    -V, --version       print the version
    -h, --help          print this help",
        elkr::VERSION
    );
}
