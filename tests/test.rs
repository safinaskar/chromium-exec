use std::ffi::CString;

fn test_raw_json_input(input: Vec<u8>, output: Vec<serde_json::Value>) {
    let child_stdin = my_libc::pipe().unwrap();
    let child_stdout = my_libc::pipe().unwrap();

    let mut actions = my_libc::posix_spawn_file_actions_init().unwrap();

    let () = my_libc::posix_spawn_file_actions_adddup2(&mut actions, child_stdin.readable.0, 0).unwrap();
    let () = my_libc::posix_spawn_file_actions_adddup2(&mut actions, child_stdout.writable.0, 1).unwrap();

    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdin.readable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdin.writable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdout.readable.0).unwrap();
    let () = my_libc::posix_spawn_file_actions_addclose(&mut actions, child_stdout.writable.0).unwrap();

    let pid = my_libc::posix_spawnp(
        &CString::new(env!("CARGO_BIN_EXE_chromium-exec")).unwrap(),
        &actions,
        [CString::new(env!("CARGO_BIN_EXE_chromium-exec")).unwrap()].iter(),
        my_libc::env_for_posix_spawn().iter()
    ).unwrap();

    drop(child_stdin.readable);
    drop(child_stdout.writable);

    let () = my_libc::write_repeatedly(child_stdin.writable.0, &std::convert::identity::<u32>(input.len().try_into().unwrap()).to_ne_bytes()).unwrap();
    let () = my_libc::write_repeatedly(child_stdin.writable.0, &input).unwrap();

    // В этом месте можно сделать drop(child_stdin.writable), но я специально не делаю этого, т. к., как я понимаю, Chromium тоже не делает этого

    for i in output {
        let mut len = [0u8; 4];
        let () = my_libc::xx_read_repeatedly(child_stdout.readable.0, &mut len).unwrap();
        let mut actual_output = vec![0u8; u32::from_ne_bytes(len).try_into().unwrap()];
        let () = my_libc::xx_read_repeatedly(child_stdout.readable.0, &mut actual_output).unwrap();
        assert_eq!(serde_json::from_slice::<'_, serde_json::Value>(&actual_output).unwrap(), i);
    }

    assert_eq!(my_libc::waitpid(pid, 0).unwrap().status, my_libc::ProcessStatus::Exited(0));
    assert_eq!(my_libc::read(child_stdout.readable.0, &mut [0u8; 1]).unwrap().len(), 0);
}

fn test(input: serde_json::Value, output: Vec<serde_json::Value>) {
    test_raw_json_input(serde_json::ser::to_vec(&input).unwrap(), output);
}

#[test]
fn tests() {
    test(
        serde_json::json!({"request":[[],"echo",["echo","a"]]}),
        vec![
            serde_json::json!({"type":"stdout","data":[97,10]}),
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["bash","-c","[ 'д' = \"$(printf '\\xd0\\xb4')\" ]"]]}),
        vec![
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[208,180],"bash",["bash","-c","[ 'д' = \"$(cat)\" ]"]]}),
        vec![
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["bash","-c","printf 'д'"]]}),
        vec![
            serde_json::json!({"type":"stdout","data":[208,180]}),
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["bash","-c","exit 10"]]}),
        vec![
            serde_json::json!({"type":"terminated","reason":"exited","code":10})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["foo","-c","exit 10"]]}),
        vec![
            serde_json::json!({"type":"terminated","reason":"exited","code":10})
        ]
    );
    test(
        serde_json::json!({"request":[[],"true",["a"]]}),
        vec![
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["a","-c","echo \"$0\""]]}),
        vec![
            serde_json::json!({"type":"stdout","data":[97,10]}),
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["bash","-c","echo a >&2"]]}),
        vec![
            serde_json::json!({"type":"stderr","data":[97,10]}),
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
    test(
        serde_json::json!({"request":[[],"bash",["bash","-c","kill -9 $$"]]}),
        vec![
            serde_json::json!({"type":"terminated","reason":"signaled","signal":9})
        ]
    );

    // Следующий тест проверяет, что мы не забыли закрыть пайп от нас к ребёнку после записи туда данных (если забыли - на этом тесте будет deadlock)
    test(
        serde_json::json!({"request":[[97,10],"cat",["cat"]]}),
        vec![
            serde_json::json!({"type":"stdout","data":[97,10]}),
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );

    // Тестим большой вход в команду. Документация Chrome устанавливает предел 4 GiB. Т. к. мы передаём массив вида [0,0,0,...], то количество элементов массива будет примерно 2 * 1024 * 1024 * 1024
    // Сперва я попробовал сгенерить требуемый JSON с помощью serde_json, но увидел ошибку "memory allocation of 68_719_476_736 bytes failed", так что, видимо, здесь придётся генерить JSON самому
    // Чуть больше 4 GiB переполняет длину. Поэтому нужно в точности 4 GiB минус один байт
    {
        let mut input = vec![];
        input.extend_from_slice(br#"{"request":[["#);
        for _ in 0 .. 2_u64 * 1024 * 1024 * 1024 / 32 - 1 {
            input.extend_from_slice(b"0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,"); // 32 нуля
        }
        input.extend_from_slice(br#"0,0,0,0,0,0,0,0,0,0,0,0,0,0],"md5sum",["md5sum"]]}"#);
        assert_eq!(input.len(), 4 * 1024 * 1024 * 1024 - 1);
        let md5: &[u8] = b"fcd21c7e5ebb7c2b3ea1461868cc1459  -\n";
        test_raw_json_input(input,
            vec![
                serde_json::json!({"type":"stdout","data":md5}),
                serde_json::json!({"type":"terminated","reason":"exited","code":0})
            ]
        );
    }
}

#[test]
fn big_output_to_chromium_exec() {
    let script = format!(r#"for((I = 0; I != {}; ++I)){{ printf a; }}; for((I = 0; I != {}; ++I)){{ printf b; }}"#, chromium_exec::CHUNK_SIZE, chromium_exec::CHUNK_SIZE);
    let aaa: &[u8] = &[b'a'; chromium_exec::CHUNK_SIZE];
    let bbb: &[u8] = &[b'b'; chromium_exec::CHUNK_SIZE];
    test(
        serde_json::json!({"request":[[],"bash",["bash","-c",script]]}),
        vec![
            serde_json::json!({"type":"stdout","data":aaa}),
            serde_json::json!({"type":"stdout","data":bbb}),
            serde_json::json!({"type":"terminated","reason":"exited","code":0})
        ]
    );
}
