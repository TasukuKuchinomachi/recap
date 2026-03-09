mod executor;
mod mcp;
mod storage;

use storage::Storage;

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  recap <command> [args...]   コマンドを実行し、結果を保存");
    eprintln!("  recap -l, --last           直前の実行結果を表示");
    eprintln!("  recap -n <N>               直近N件の一覧を表示");
    eprintln!("  recap -s, --show <ID>      指定IDの実行結果を表示");
    eprintln!("  recap --mcp                MCP サーバーとして起動");
}

fn show_entry_detail(storage: &Storage, entry: &storage::RunEntry) {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "#{:03} | {} | exit: {}",
        entry.id, entry.cmd, entry.exit
    );
    println!("{}", entry.at.format("%Y-%m-%d %H:%M:%S"));
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    match storage.read_log(entry.id) {
        Ok(log) => print!("{}", log),
        Err(e) => eprintln!("[recap] ログの読み込みに失敗: {}", e),
    }
}

fn show_list(storage: &Storage, n: usize) {
    match storage.get_last_n(n) {
        Ok(entries) => {
            println!(
                " {:>3} | {:>4} | {:>19} | COMMAND",
                "ID", "EXIT", "TIME"
            );
            println!("-----+------+---------------------+------------------");
            for entry in entries.iter().rev() {
                println!(
                    " {:03} | {:>4} | {} | {}",
                    entry.id,
                    entry.exit,
                    entry.at.format("%Y-%m-%d %H:%M:%S"),
                    entry.cmd
                );
            }
        }
        Err(e) => eprintln!("[recap] エラー: {}", e),
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
