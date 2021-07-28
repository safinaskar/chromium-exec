/*
{
  "name": "chromium_exec",
  "description": "chromium-exec",
  "path": "/usr/local/bin/chromium-exec",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://aaaaaaaaaaaaa/"]
}
*/

// { "request": [[<input byte>, <input byte>, ...], <executable>, [<argv[0]>, <argv[1]>, ...]] }

// Сначала пишет весь stdin в процесс и потом вычитывает stdout и stderr, это может привести к deadlock'у
// Было бы неплохо писать в stdout хотя бы о некоторых ошибках, например, об отсутствии бинарника, вместо "panic!"
// Эта программа только для UNIX-like, все зависимости от окружения прямо указаны
// Протокол изначально придумывался для самописного JSON-парсера, теперь протокол JSON'а от Chrome к этому бинарю можно сделать более естественным

use std::io::Read;
use std::io::Write;
use std::os::unix::process::CommandExt;
use std::os::unix::process::ExitStatusExt;

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    request: (Vec<u8>, String, Vec<String>)
}

fn send(json: &serde_json::Value) {
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let vec = serde_json::ser::to_vec(json).unwrap();
    stdout.write_all(&(vec.len() as u32).to_ne_bytes()).unwrap();
    stdout.write_all(&vec).unwrap();
}

fn main() {
    let Input { request: (input_for_exec, exec, args) } = {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();
        let mut len = [0u8; 4];
        stdin.read_exact(&mut len).unwrap();
        let mut input = vec![0u8; u32::from_ne_bytes(len).try_into().unwrap()];
        stdin.read_exact(&mut input).unwrap(); // Надеюсь, мы не попытаемся прочитать здесь лишние байты (это может привести к зависанию)
        // Опыт показывает, что не нужно пытаться прочитать тут ещё байт, чтобы выяснить, что у нас EOF. Это приводит к зависанию
        serde_json::from_slice(&input).unwrap()
    };

    if args.len() == 0 {
        panic!();
    };

    let child = std::process::Command::new(exec)
        .arg0(&args[0])
        .args(&args[1..])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_ref().unwrap().write_all(&input_for_exec).unwrap();

    let output = child.wait_with_output().unwrap();

    // В идеале нужно слать данные из stdout и stderr в том порядке, в котором они приходят. Но я шлю сперва stdout, а потом stderr. Протокол сделан таким, чтобы можно было позже переделать. В частности, расширение должно предполагать, что данные могут идти в любом порядке

    // Лимит, указанный в документации: 1 MB, т. е. 1024 * 1024
    // Нужно:
    // array_size * 4 + 100 <= 1024 * 1024
    // array_size * 4 <= 1024 * 1024 - 100
    // array_size <= (1024 * 1024 - 100) / 4

    for chunk in output.stdout.chunks((1024 * 1024 - 100) / 4) {
        send(&serde_json::json!({"type": "stdout", "data": chunk}));
    };

    for chunk in output.stderr.chunks((1024 * 1024 - 100) / 4) {
        send(&serde_json::json!({"type": "stderr", "data": chunk}));
    };

    if let Some(sig) = output.status.signal() {
        send(&serde_json::json!({"type": "terminated", "reason": "signaled", "signal": sig}));
    } else if let Some(code) = output.status.code() {
        send(&serde_json::json!({"type": "terminated", "reason": "exited", "code": code}));
    } else {
        send(&serde_json::json!({"type": "terminated", "reason": "unknown"}));
    };
}
