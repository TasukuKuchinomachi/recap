mod executor;
mod mcp;
mod storage;

use comfy_table::{presets, Attribute, Cell, Color, ContentArrangement, Table};
use console::{style, Term};
use storage::Storage;

fn print_usage() {
    let term = Term::stderr();
    let _ = term.write_line(&format!("{}", style("Usage:").bold()));
    let _ = term.write_line(&format!(
        "  {}   コマンドを実行し、結果を保存 (パイプ可)",
        style("recap <command...>").green()
    ));
    let _ = term.write_line(&format!(
        "  {}           直前の実行結果を表示",
        style("recap -l, --last").green()
    ));
    let _ = term.write_line(&format!(
        "  {}               直近N件の一覧を表示",
        style("recap -n <N>").green()
    ));
    let _ = term.write_line(&format!(
        "  {}      指定IDの実行結果を表示",
        style("recap -s, --show <ID>").green()
    ));
    let _ = term.write_line(&format!(
        "  {}                MCP サーバーとして起動",
        style("recap --mcp").green()
    ));
    let _ = term.write_line(&format!(
        "  {}       シェル統合を初期化 (bash/zsh/fish)",
        style("recap init <shell>").green()
    ));
}

fn print_shell_init(shell: &str) {
    let recap_bin = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "recap".to_string());

    match shell {
        "bash" | "zsh" => {
            println!(
                r#"r() {{
  local cmd
  cmd="$*"
  {bin} --exec "$cmd"
}}"#,
                bin = recap_bin
            );
        }
        "fish" => {
            println!(
                r#"function r
  set -l cmd (string join " " $argv)
  {bin} --exec "$cmd"
end"#,
                bin = recap_bin
            );
        }
        _ => {
            eprintln!(
                "{} 未対応のシェル: {} (bash, zsh, fish に対応)",
                style("[recap]").red(),
                shell
            );
            std::process::exit(1);
        }
    }
}

fn show_entry_detail(storage: &Storage, entry: &storage::RunEntry) {
    let exit_style = if entry.exit == 0 {
        style(format!("exit: {}", entry.exit)).green()
    } else {
        style(format!("exit: {}", entry.exit)).red()
    };

    println!(
        "{} {} {}  {}",
        style(format!("#{:03}", entry.id)).cyan().bold(),
        style(&entry.cmd).bold(),
        exit_style,
        style(entry.at.format("%Y-%m-%d %H:%M:%S")).dim(),
    );
    println!("{}", style("─".repeat(50)).dim());
    match storage.read_log(entry.id) {
        Ok(log) => print!("{}", log),
        Err(e) => eprintln!("{} ログの読み込みに失敗: {}", style("[recap]").red(), e),
    }
}

fn show_list(storage: &Storage, n: usize) {
    match storage.get_last_n(n) {
        Ok(entries) if entries.is_empty() => {
            eprintln!("{} 実行履歴がありません", style("[recap]").yellow());
        }
        Ok(entries) => {
            let mut table = Table::new();
            table
                .load_preset(presets::UTF8_FULL_CONDENSED)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("ID").add_attribute(Attribute::Bold),
                    Cell::new("EXIT").add_attribute(Attribute::Bold),
                    Cell::new("TIME").add_attribute(Attribute::Bold),
                    Cell::new("COMMAND").add_attribute(Attribute::Bold),
                ]);

            for entry in entries.iter().rev() {
                let exit_cell = if entry.exit == 0 {
                    Cell::new(entry.exit).fg(Color::Green)
                } else {
                    Cell::new(entry.exit).fg(Color::Red)
                };
                table.add_row(vec![
                    Cell::new(format!("{:03}", entry.id)).fg(Color::Cyan),
                    exit_cell,
                    Cell::new(entry.at.format("%Y-%m-%d %H:%M:%S").to_string()),
                    Cell::new(&entry.cmd),
                ]);
            }

            println!("{table}");
        }
        Err(e) => eprintln!("{} エラー: {}", style("[recap]").red(), e),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let storage = Storage::new();

    match args[0].as_str() {
        "-l" | "--last" => match storage.get_last() {
            Ok(Some(entry)) => show_entry_detail(&storage, &entry),
            Ok(None) => eprintln!("[recap] 実行履歴がありません"),
            Err(e) => eprintln!("[recap] エラー: {}", e),
        },
        "-n" => {
            let n: usize = args
                .get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
            show_list(&storage, n);
        }
        "-s" | "--show" => {
            let id: u64 = match args.get(1).and_then(|s| s.parse().ok()) {
                Some(id) => id,
                None => {
                    eprintln!("[recap] IDを指定してください");
                    std::process::exit(1);
                }
            };
            match storage.get_by_id(id) {
                Ok(Some(entry)) => show_entry_detail(&storage, &entry),
                Ok(None) => eprintln!("[recap] ID {} の記録が見つかりません", id),
                Err(e) => eprintln!("[recap] エラー: {}", e),
            }
        }
        "-h" | "--help" => {
            print_usage();
        }
        "init" => {
            let shell = args.get(1).map(|s| s.as_str()).unwrap_or("bash");
            print_shell_init(shell);
        }
        "--exec" => {
            let cmd = args[1..].join(" ");
            let exit_code = match executor::execute_and_record(&[cmd]) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("[recap] 実行エラー: {}", e);
                    1
                }
            };
            std::process::exit(exit_code);
        }
        "--mcp" => {
            if let Err(e) = mcp::run_mcp_server() {
                eprintln!("[recap] MCP サーバーエラー: {}", e);
                std::process::exit(1);
            }
        }
        _ => {
            let exit_code = match executor::execute_and_record(&args) {
                Ok(code) => code,
                Err(e) => {
                    eprintln!("[recap] 実行エラー: {}", e);
                    1
                }
            };
            std::process::exit(exit_code);
        }
    }
}
