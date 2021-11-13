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
// Протокол изначально придумывался для самописного JSON-парсера, теперь протокол JSON'а от Chrome к этому бинарю можно сделать более естественным

#![deny(unsafe_code)]

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    request: (Vec<u8>, String, Vec<String>)
}

fn send(json: &serde_json::Value) {
    let vec = serde_json::ser::to_vec(json).unwrap();
    let () = my_libc::write_repeatedly(1, &(vec.len() as u32).to_ne_bytes()).unwrap();
    let () = my_libc::write_repeatedly(1, &vec).unwrap();
}

fn main() {
    let Input { request: (input_for_exec, exec, args) } = {
        let mut len = [0u8; 4];
        let () = my_libc::xx_read_repeatedly(0, &mut len).unwrap();
        let mut input = vec![0u8; u32::from_ne_bytes(len).try_into().unwrap()];
        let () = my_libc::xx_read_repeatedly(0, &mut input).unwrap();

        // Опыт показывает, что не нужно пытаться прочитать тут ещё один байт, чтобы выяснить, что у нас EOF. Это приводит к зависанию

        serde_json::from_slice(&input).unwrap()
    };

    if args.len() == 0 {
        panic!();
    };

    let child_stdin = my_libc::pipe().unwrap();
    let child_stdout = my_libc::pipe().unwrap();
    let child_stderr = my_libc::pipe().unwrap();

    let mut actions = my_libc::posix_spawn_file_actions_init().unwrap();

    let () = my_libc::posix_spawn_file_actions_adddup2(&mut actions, child_stdin.readable.0, 0).unwrap();
    let () = my_libc::posix_spawn_file_actions_adddup2(&mut actions, child_stdout.writable.0, 1).unwrap();
    let () = my_libc::posix_spawn_file_actions_adddup2(&mut actions, child_stderr.writable.0, 2).unwrap();

    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdin.readable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdin.writable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdout.readable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdout.writable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stderr.readable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stderr.writable.0).unwrap();

    let pid = my_libc::posix_spawnp(&std::ffi::CString::new(exec).unwrap(), &actions, args.into_iter().map(|s|std::ffi::CString::new(s).unwrap()).collect::<Vec<_>>().iter(), my_libc::env_for_posix_spawn().iter()).unwrap();

    drop(child_stdin.readable);
    drop(child_stdout.writable);
    drop(child_stderr.writable);

    let () = my_libc::write_repeatedly(child_stdin.writable.0, &input_for_exec).unwrap();
    drop(child_stdin.writable);

    // Лимит, указанный в документации: 1 MB, т. е. 1024 * 1024
    // Нужно:
    // array_size * 4 + 100 <= 1024 * 1024
    // array_size * 4 <= 1024 * 1024 - 100
    // array_size <= (1024 * 1024 - 100) / 4

    fn send_chunks(fd: my_libc::FD, name: &str) {
        loop {
            let mut buf = [0u8; (1024 * 1024 - 100) / 4];
            let got = my_libc::read_repeatedly(fd.0, &mut buf).unwrap();

            if got.is_empty() {
                break;
            }

            send(&serde_json::json!({"type": name, "data": got}));
        }
    }

    // В идеале нужно слать данные из stdout и stderr в том порядке, в котором они приходят. Но я шлю сперва stdout, а потом stderr. Протокол сделан таким, чтобы можно было позже переделать. В частности, расширение должно предполагать, что данные могут идти в любом порядке
    send_chunks(child_stdout.readable, "stdout");
    send_chunks(child_stderr.readable, "stderr");

    match my_libc::waitpid(pid, 0).unwrap().status {
        my_libc::ProcessStatus::Exited(code) =>
            send(&serde_json::json!({"type": "terminated", "reason": "exited", "code": code})),
        my_libc::ProcessStatus::Signaled { termsig: signal, coredump: _ } =>
            send(&serde_json::json!({"type": "terminated", "reason": "signaled", "signal": signal})),
        _ =>
            send(&serde_json::json!({"type": "terminated", "reason": "unknown"})),
    }
}
