//! Find the working store by name while all operands and lists keep their origin.
use super::{SALT, Store, WHEN, answer, fixed_store, save_morning};
use std::fs;

const DOT: &str = r#"{"vertex":"kiln","from":[{"store":"."}],"fold":{"kilns":{"count":true}}}"#;
const TWO: &str = r#"{"vertex":"kiln","from":[{"store":"."},{"store":"backups"}],"fold":{"kilns":{"count":true}}}"#;

fn sibling(shop: &Store, name: &str) -> Store {
    let store = Store {
        dir: shop.scratch.join(name),
        scratch: shop.scratch.clone(),
    };
    fs::create_dir_all(&store.dir).expect("fixture directory");
    fs::write(store.dir.join("device-key"), "0".repeat(52)).expect("matching signer");
    store
}

fn setup(label: &str) -> (Store, Store) {
    let shop = fixed_store(label);
    let yard = sibling(&shop, "yard");
    yard.ok(&["init"]);
    fs::write(
        shop.scratch.join("stores.json"),
        r#"{"yard":"yard","backups":"store"}"#,
    )
    .expect("parent list");
    (shop, yard)
}

fn notice(yard: &Store, source: &str) -> String {
    let dir = yard.dir.canonicalize().expect("yard directory");
    format!("writing --in yard to {} (bound by {source})", dir.display())
}

#[test]
fn selectors_are_checked_before_any_store_is_opened() {
    let shop = Store::scratch("workshop-selectors");
    let absent = shop.dir.join("missing");
    for verb in [
        "emit",
        "erase",
        "tick",
        "import",
        "get",
        "of",
        "export",
        "check",
        "ls",
        "read",
        "fold",
        "init",
        "whoami",
        "shadow-import",
        "--help",
        "--version",
    ] {
        for flags in [
            &["--in", "yard", "--in", "yard"][..],
            &["--group"],
            &["--in", "yard", "--group"],
        ] {
            let mut args = vec!["-C", absent.to_str().expect("path")];
            args.extend(flags);
            args.push(verb);
            let (code, out, _) = answer(&shop, &args, "");
            assert_eq!((code, out.as_str()), (2, ""), "{args:?}");
        }
    }
    for verb in ["init", "whoami", "shadow-import", "--help", "--version"] {
        assert_eq!(shop.fails(&["--in", "yard", verb], "").0, 2);
    }
    assert_eq!(shop.fails(&["--in"], "").0, 2);
    assert!(!absent.exists());
    assert!(!shop.dir.join("device-key").exists());
}

#[test]
fn named_writes_report_the_target_and_reads_match_directory_stdout() {
    let (shop, yard) = setup("kiln-named-writes");
    let local = shop.scratch.join("stores.json");
    let expected_notice = notice(&yard, local.to_str().expect("path"));
    let emit = [
        "emit",
        "kiln",
        "--of",
        "kiln:east",
        "--salt",
        SALT,
        "--emitted",
        WHEN,
    ];
    let invoke = |args: &[&str]| answer(&shop, &[&["--in", "yard"][..], args].concat(), "");

    // The identical store and flags, restored to its original empty holding.
    let (code, id, msg) = invoke(&emit);
    assert_eq!((code, msg.as_str()), (0, expected_notice.as_str()));
    let signed = yard.run(&["export"], "").stdout;
    for file in yard.segments() {
        fs::remove_file(file).expect("restore empty holding");
    }
    let direct = yard.run(&emit, "");
    assert!(direct.status.success() && direct.stderr.is_empty());
    assert_eq!(id.as_bytes(), direct.stdout);
    assert_eq!(yard.run(&["export"], "").stdout, signed);
    let id = id.trim_end();

    save_morning(&yard, DOT);
    let (code, tick, msg) = invoke(&["tick", "morning"]);
    assert_eq!((code, msg.as_str()), (0, expected_notice.as_str()));
    for args in [
        &["ls"][..],
        &["ls", "kiln:"],
        &["get", id],
        &["of", "kiln:east"],
        &["check"],
        &["export"],
        &["tick", "--check", tick.trim_end()],
        &["read", "morning"],
    ] {
        let (code, stdout, stderr) = invoke(args);
        let direct = yard.run(args, "");
        assert!(direct.status.success(), "{args:?}");
        assert_eq!(code, 0, "{args:?}");
        assert_eq!(stdout.as_bytes(), direct.stdout, "{args:?}");
        assert!(stderr.is_empty(), "{args:?}: {stderr}");
    }

    let (code, _, msg) = invoke(&["erase", id]);
    assert_eq!((code, msg.as_str()), (0, expected_notice.as_str()));
    assert!(!super::has(&yard.json(&["get", id]), &[], "payload"));
    assert_eq!(shop.segments(), Vec::<std::path::PathBuf>::new());
}

#[test]
fn file_operands_and_all_names_keep_the_origin() {
    let (shop, yard) = setup("kiln-origin");
    shop.ok(&["emit", "kiln"]);
    yard.changeset("{\"kind\":\"yard\"}\n{\"kind\":\"potter\"}");
    let origin = sibling(&shop, "store/potter");
    // Searching the selected store or this subdirectory for lists would fail.
    fs::write(yard.dir.join("stores.json"), "{").expect("target decoy");
    fs::write(origin.dir.join("stores.json"), "{").expect("subdirectory decoy");
    fs::write(origin.dir.join("kiln.json"), TWO).expect("relative spec");
    save_morning(&yard, TWO);
    for args in [&["read", "morning"][..], &["fold", "kiln.json"]] {
        let command = [&["--in", "yard"][..], args].concat();
        assert_eq!(
            answer(&origin, &command, ""),
            (
                0,
                "{\"kilns\":{\"unread\":0,\"value\":3}}\n".into(),
                String::new()
            )
        );
        let post = [&command[..], &["--store", ".=.."]].concat();
        let pre = [&["--store", ".=.."][..], &command].concat();
        assert_eq!(origin.ok(&post), r#"{"kilns":{"unread":0,"value":1}}"#);
        assert_eq!(origin.ok(&post), origin.ok(&pre));
        let both = [&pre[..], &["--store", ".=../../yard"]].concat();
        assert_eq!(origin.ok(&both), origin.ok(&command));
    }
    let local = shop.scratch.join("stores.json");
    let msg = notice(&yard, local.to_str().expect("path"));
    let (code, tick, stderr) = answer(&origin, &["--in", "yard", "tick", "kiln.json"], "");
    assert_eq!((code, stderr.as_str()), (0, msg.as_str()));
    assert!(
        yard.run(&["tick", "--check", tick.trim_end()], "")
            .status
            .success()
    );
    fs::write(
        origin.dir.join("backups.jsonl"),
        shop.run(&["export"], "").stdout,
    )
    .expect("import file");
    let (code, _, stderr) = answer(&origin, &["--in", "yard", "import", "backups.jsonl"], "");
    assert_eq!((code, stderr.as_str()), (0, msg.as_str()));
    assert_eq!(yard.ok(&["export", "--kind", "kiln"]), shop.ok(&["export"]));
}

#[test]
fn precedence_exact_paths_and_outside_origin() {
    let (shop, yard) = setup("yard-precedence");
    let outside = sibling(&shop, "workshop");
    let config = shop.scratch.join("home/xdg-config/loops/stores.json");
    fs::write(&config, r#"{"yard":"../../../store"}"#).expect("user list");
    let emit = ["--in", "yard", "emit", "potter"];
    // From outside: the parent of the origin does not supply a list.
    let user_notice = format!(
        "writing --in yard to {} (bound by {})",
        shop.dir.display(),
        config.display()
    );
    let (code, _, msg) = answer(&outside, &emit, "");
    assert_eq!((code, msg), (0, user_notice.clone()));
    let local = outside.dir.join("stores.json");
    fs::write(&local, r#"{"yard":"../yard"}"#).expect("outside local list");
    assert_eq!(
        answer(&outside, &emit, "").2,
        notice(&yard, local.to_str().expect("path"))
    );
    let root = shop.dir.join("stores.json");
    fs::write(&root, "{}").expect("root hides parent list");
    assert_eq!(answer(&shop, &emit, "").2, user_notice);
    fs::write(&root, r#"{"yard":"../yard"}"#).expect("root winner");
    assert_eq!(
        answer(&shop, &emit, "").2,
        notice(&yard, root.to_str().expect("path"))
    );
    let command = [&["--store", "yard=.", "--store", "yard=../yard"][..], &emit].concat();
    assert_eq!(
        answer(&shop, &command, "").2,
        notice(&yard, "--store yard=../yard")
    );
    fs::create_dir(outside.dir.join("backups")).expect("directory below outside origin");
    let by_name = shop.ok(&[
        "--store",
        "yard=backups/../../yard",
        "-C",
        outside.dir.to_str().expect("outside path"),
        "--in",
        "yard",
        "ls",
    ]);
    assert_eq!(by_name, yard.ok(&["ls"]));
    let child = yard.dir.join("potter");
    fs::create_dir(&child).expect("child directory");
    let (code, out, msg) = answer(
        &shop,
        &["--store", "kiln=../yard/potter", "--in", "kiln", "ls"],
        "",
    );
    assert_eq!((code, out.as_str()), (1, ""));
    assert!(msg.starts_with("refused: store-unknown: kiln:"));
    assert_eq!(
        shop.fails(&["--in", "unbound", "ls"], ""),
        (1, "refused: store-unknown: unbound".into())
    );
}

#[cfg(unix)]
#[test]
fn aliases_and_symlinks_share_one_store() {
    let (shop, yard) = setup("yard-aliases");
    std::os::unix::fs::symlink(&yard.dir, shop.dir.join("potter")).expect("store alias");
    yard.ok(&["emit", "kiln"]);
    let args = [
        "--store",
        "yard=potter",
        "--store",
        "backups=../yard",
        "--in",
        "yard",
        "fold",
        "-",
    ];
    assert_eq!(
        shop.ok_with(&args, TWO),
        r#"{"kilns":{"unread":0,"value":1}}"#
    );
}

#[test]
fn a_named_writer_keeps_its_opened_store_after_the_notice() {
    use std::{
        io::{BufRead, BufReader, Write},
        process::Stdio,
    };

    let (shop, yard) = setup("yard-open-import");
    shop.ok(&["emit", "kiln"]);
    let input = shop.run(&["export"], "").stdout;
    let mut child = shop
        .command(&["--in", "yard", "import", "-"])
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start import");
    let local = shop.scratch.join("stores.json");
    // If the notice moves after stdin, fail this test instead of deadlocking CI.
    let mut stderr = BufReader::new(child.stderr.take().expect("child stderr"));
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        stderr.read_line(&mut line).expect("destination notice");
        let _ = tx.send(line);
    });
    let line = rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap_or_else(|e| {
            let _ = child.kill();
            let _ = child.wait();
            panic!("import did not announce its destination before reading stdin: {e}");
        });
    assert_eq!(
        line,
        format!("{}\n", notice(&yard, local.to_str().expect("list path")))
    );
    fs::write(local, r#"{"yard":"store"}"#).expect("rebind yard while waiting for input");
    child
        .stdin
        .take()
        .expect("child stdin")
        .write_all(&input)
        .expect("import input");
    assert!(
        child
            .wait_with_output()
            .expect("import exits")
            .status
            .success()
    );
    assert_eq!(yard.run(&["export"], "").stdout, input);
}

#[test]
fn dot_bindings_override_the_selected_store_and_dot_can_select_the_origin() {
    let (shop, yard) = setup("workshop-dot");
    shop.ok(&["emit", "kiln"]);
    yard.changeset("{\"kind\":\"yard\"}\n{\"kind\":\"potter\"}");
    let fold = ["--in", "yard", "fold", "-", "--plain"];
    assert_eq!(shop.ok_with(&fold, DOT), "kilns  2");
    assert_eq!(
        answer(&shop, &["--in", ".", "emit", "potter"], "").2,
        format!(
            "writing --in . to {} (bound by current store)",
            shop.dir.display()
        )
    );
    shop.ok(&["emit", "kiln"]);
    fs::write(
        shop.scratch.join("home/xdg-config/loops/stores.json"),
        r#"{".":"../../../store"}"#,
    )
    .expect("user dot");
    assert_eq!(shop.ok_with(&fold, DOT), "kilns  3");
    fs::write(
        shop.dir.join("stores.json"),
        r#"{".":"../yard","yard":"../yard"}"#,
    )
    .expect("local dot");
    assert_eq!(shop.ok_with(&fold, DOT), "kilns  2");
    let override_dot = [&["--store", ".=."][..], &fold].concat();
    assert_eq!(shop.ok_with(&override_dot, DOT), "kilns  3");
}

#[test]
fn global_binding_alone_does_not_select_a_store() {
    let shop = Store::scratch("workshop-binding");
    fs::write(shop.dir.join("stores.json"), "{").expect("broken local list");
    for verb in [
        &["init"][..],
        &["whoami", "--init"],
        &["whoami"],
        &["ls"],
        &["--version"],
    ] {
        let args = [&["--store", "yard=absent"][..], verb].concat();
        assert!(shop.run(&args, "").status.success(), "{args:?}");
    }
    assert_eq!(
        shop.fails(&["--store", "yard=absent", "shadow-import"], ""),
        (2, "shadow-import requires a group file".to_owned())
    );
    assert_eq!(shop.fails(&["--store", "yard=", "ls"], "").0, 2);
}
