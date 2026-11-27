use super::*;
use rusqlite::params;

fn sample() -> Value {
    json!({"meta":{"source_provider":"fixture","source_id":"original","title":"Проверка 🐱","directory":""},"entries":[
    {"kind":"user","text":"Задача: сохранить контекст","ts":"2026-11-19T10:00:00Z"},
    {"kind":"tool","subtype":"thinking","text":"План анализа","ts":"2026-11-19T10:00:01Z"},
    {"kind":"assistant","text":"Начинаю работу","ts":"2026-11-19T10:00:02Z"},
    {"kind":"tool","subtype":"tool_output","tool":"write_file","text":"Файл создан 🐱","status":"ok","ts":"2026-11-19T10:00:03Z"},
    {"kind":"system","subtype":"instructions","text":"Использовать UTF-8","ts":"2026-11-19T10:00:04Z"},
    {"kind":"compact","text":"Прежние решения сохранены","ts":"2026-11-19T10:00:05Z"},
    {"kind":"error","text":"Синтетическая ошибка","ts":"2026-11-19T10:00:06Z"},
    {"kind":"user","text":"Продолжай","ts":"2026-11-19T10:00:07Z"},
    {"kind":"assistant","text":"Готово","ts":"2026-11-19T10:00:08Z"}]})
}

#[test]
fn lexical_normpath_preserves_native_roots_and_collapses_parent_components() {
    for (path, expected) in [
        ("", "."),
        ("missing/../project/", "project"),
        ("../../missing/../project//", "../../project"),
        ("/../missing/../project/./", "/project"),
        ("//host/share/../project/", "//host/project"),
        ("///host//project/", "/host/project"),
        (r"dir\name/../project", "project"),
    ] {
        assert_eq!(lexical_normpath_for(path, false), expected, "{path}");
    }
    for (path, expected) in [
        ("", "."),
        ("missing/../project/", "project"),
        (r"C:\missing\..\проект\", r"C:\проект"),
        (r"C:missing\..\project\", r"C:project"),
        (r"C:..\project", r"C:..\project"),
        (r"C:\..\..\project", r"C:\project"),
        (
            r"\\server\share\missing\..\..\project\",
            r"\\server\share\project",
        ),
        (r"\\server\share\", r"\\server\share\"),
        (r"\\?\C:\missing\..\project\", r"\\?\C:\project"),
        (
            r"\\?\UNC\server\share\missing\..\project\",
            r"\\?\UNC\server\share\project",
        ),
        (r"\\.\pipe\missing\..\project", r"\\.\pipe\project"),
    ] {
        assert_eq!(lexical_normpath_for(path, true), expected, "{path}");
    }
}

#[test]
fn strip_long_preserves_unc_roots_and_non_drive_namespaces() {
    assert_eq!(strip_long(r"\\?\C:\Users\пример"), r"C:\Users\пример");
    assert_eq!(
        strip_long(r"\\?\UNC\server\share\project"),
        r"\\server\share\project"
    );
    assert_eq!(
        strip_long(r"\\?\unc\server\share\project"),
        r"\\server\share\project"
    );
    assert_eq!(
        strip_long(r"\\?\Volume{example}\project"),
        r"\\?\Volume{example}\project"
    );
}

#[test]
fn claude_import_normalizes_cwd_before_project_slug_and_history() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("claude");
    let previous = d.path().join("previous");
    let project = d.path().join("project");
    fs::create_dir_all(&previous).unwrap();
    fs::create_dir_all(&project).unwrap();
    let mut doc = sample();
    doc["meta"]["directory"] = json!(format!("{}/", previous.join("../project").display()));
    let result = claude::import(&doc, &root, None, true, false, None).unwrap();
    let cid = result["cid"].as_str().unwrap();
    let path = recent_files(&root.join("projects"), 2, "jsonl")
        .into_iter()
        .find(|p| p.file_stem().unwrap().to_string_lossy() == cid)
        .unwrap();
    let expected = strip_long(&project.canonicalize().unwrap().to_string_lossy());
    let text = fs::read_to_string(path).unwrap();
    let rows: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(rows.iter().any(|row| row["cwd"] == expected));
    assert!(rows
        .iter()
        .filter(|row| !row["cwd"].is_null())
        .all(|row| row["cwd"] == expected));
    let history = fs::read_to_string(root.join("history.jsonl")).unwrap();
    let history: Value = serde_json::from_str(history.trim()).unwrap();
    assert_eq!(history["project"], expected);
}

#[test]
fn claude_import_uses_existing_local_workspaces_for_missing_or_foreign_paths() {
    for directory in [
        format!(r"Z:\missing-{}\Проект", uuid()),
        format!("/missing-{}/project", uuid()),
        format!("relative-{}", uuid()),
    ] {
        let d = tempfile::tempdir().unwrap();
        let root = d.path().join("claude");
        let mut doc = sample();
        doc["meta"]["directory"] = json!(directory);
        let result = claude::import(&doc, &root, None, true, false, None).unwrap();
        let path = recent_files(&root.join("projects"), 2, "jsonl").remove(0);
        let rows = crate::storage::read_jsonl(&path).unwrap();
        let directory = rows.iter().find_map(|row| row["cwd"].as_str()).unwrap();
        let directory = Path::new(directory);
        assert!(directory.is_absolute());
        assert!(directory.is_dir());
        assert_eq!(
            directory.parent().unwrap().canonicalize().unwrap(),
            root.join("workspaces").canonicalize().unwrap()
        );
        assert_eq!(result["entries"], doc["entries"].as_array().unwrap().len());
    }
}

#[test]
fn claude_import_accepts_deep_unicode_workspaces_without_oversized_project_components() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("claude");
    let directory = d
        .path()
        .join("Русский проект")
        .join("a".repeat(80))
        .join("b".repeat(80))
        .join("c".repeat(80));
    fs::create_dir_all(&directory).unwrap();
    let mut doc = sample();
    doc["meta"]["directory"] = json!(directory);
    let result = claude::import(&doc, &root, None, true, false, None).unwrap();
    let path = recent_files(&root.join("projects"), 2, "jsonl").remove(0);
    assert!(path.parent().unwrap().file_name().unwrap().len() <= 207);
    let expected = strip_long(&directory.canonicalize().unwrap().to_string_lossy());
    let rows = crate::storage::read_jsonl(&path).unwrap();
    assert!(rows.iter().any(|row| row["cwd"] == expected));
    assert_eq!(result["entries"], doc["entries"].as_array().unwrap().len());
    assert!(path
        .parent()
        .unwrap()
        .join(result["cid"].as_str().unwrap())
        .join("custom-title.json")
        .is_file());
}

#[test]
fn claude_import_rolls_back_history_and_session_on_desktop_write_failure() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("claude");
    let retained = root
        .join("projects")
        .join("retained")
        .join("existing.jsonl");
    fs::create_dir_all(retained.parent().unwrap()).unwrap();
    let transcript =
        b"{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"retained\"}}\n";
    fs::write(&retained, transcript).unwrap();
    let history = root.join("history.jsonl");
    let original = b"{\"sessionId\":\"existing\",\"display\":\"retained\"}";
    fs::write(&history, original).unwrap();
    let desktop = d.path().join("desktop-file");
    fs::write(&desktop, b"retained desktop data").unwrap();
    let error = claude::import(&sample(), &root, None, true, false, Some(&desktop)).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("Claude Desktop"), "{message}");
    assert!(
        message.contains(&desktop.to_string_lossy().to_string()),
        "{message}"
    );
    assert_eq!(fs::read(&history).unwrap(), original);
    assert_eq!(fs::read(&retained).unwrap(), transcript);
    assert_eq!(fs::read(&desktop).unwrap(), b"retained desktop data");
    assert_eq!(
        recent_files(&root.join("projects"), 2, "jsonl"),
        vec![retained]
    );
    assert!(recent_files(&root.join("projects"), 3, "json").is_empty());
    assert_eq!(fs::read_dir(root.join("workspaces")).unwrap().count(), 0);
}

#[test]
fn claude_import_reports_history_path_without_replacing_unreadable_history() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("claude");
    let history = root.join("history.jsonl");
    fs::create_dir_all(&history).unwrap();
    fs::write(history.join("retained"), b"retained").unwrap();
    let error = claude::import(&sample(), &root, None, true, false, None).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("прочитать историю Claude"), "{message}");
    assert!(
        message.contains(&history.to_string_lossy().to_string()),
        "{message}"
    );
    assert_eq!(fs::read(history.join("retained")).unwrap(), b"retained");
    assert!(recent_files(&root.join("projects"), 2, "jsonl").is_empty());
    assert!(recent_files(&root.join("projects"), 3, "json").is_empty());
    assert_eq!(fs::read_dir(root.join("workspaces")).unwrap().count(), 0);
}

#[test]
fn antigravity_import_preserves_protobuf_summary_fts_and_transcripts() {
    for cli in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let result = antigravity::import(&sample(), root, cli, None, true).unwrap();
        let cid = result["cid"].as_str().unwrap();
        let c = readonly(&root.join("conversations").join(format!("{cid}.db"))).unwrap();
        assert_eq!(
            c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert_eq!(
            c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
                .unwrap()
                .to_lowercase(),
            "wal"
        );
        assert_eq!(
            c.query_row("PRAGMA page_size", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            32768
        );
        assert_eq!(
            c.query_row("SELECT COUNT(*) FROM steps", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            8
        );
        let (trajectory, source) = c
            .query_row(
                "SELECT trajectory_id,source FROM trajectory_meta",
                [],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .unwrap();
        assert_eq!(source, if cli { 17 } else { 1 });
        let blob = c
            .query_row(
                "SELECT step_payload FROM steps WHERE step_type=15 ORDER BY idx LIMIT 1",
                [],
                |r| r.get::<_, Vec<u8>>(0),
            )
            .unwrap();
        assert_eq!(
            crate::pbwire::string_at(&blob, &[20, 3]).as_deref(),
            Some("План анализа")
        );
        assert_eq!(
            crate::pbwire::string_at(&blob, &[20, 8]).as_deref(),
            Some("Начинаю работу")
        );
        let summary = readonly(&root.join("conversation_summaries.db")).unwrap();
        let (n, raw) = summary
            .query_row(
                "SELECT step_count,raw_summary FROM conversation_summaries WHERE conversation_id=?",
                [cid],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)),
            )
            .unwrap();
        assert_eq!(n, 8);
        assert_eq!(
            crate::pbwire::string_at(&raw, &[4]).as_deref(),
            Some(trajectory.as_str())
        );
        let offsets: String = summary
            .query_row(
                "SELECT agent_step_byte_offsets FROM conversations_fts WHERE conversation_id=?",
                [cid],
                |r| r.get(0),
            )
            .unwrap();
        let offsets: Vec<[usize; 2]> = serde_json::from_str(&offsets).unwrap();
        assert_eq!(offsets[1][1], "Начинаю работу\n".len());
        let full = fs::read(
            root.join("brain")
                .join(cid)
                .join(".system_generated/logs/transcript_full.jsonl"),
        )
        .unwrap();
        assert_eq!(
            full,
            fs::read(
                root.join("brain")
                    .join(cid)
                    .join(".system_generated/logs/transcript.jsonl")
            )
            .unwrap()
        );
        assert_eq!(
            full,
            fs::read(
                root.join("brain")
                    .join(cid)
                    .join(".system_generated/logs/chunks/transcript_full/00000000.jsonl")
            )
            .unwrap()
        );
        let lines: Vec<Value> = full
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|b| serde_json::from_slice(b).unwrap())
            .collect();
        assert_eq!(
            lines[0]["content"],
            "<USER_REQUEST>\nЗадача: сохранить контекст\n</USER_REQUEST>"
        );
        assert_eq!(lines[1]["thinking"], "План анализа");
        assert!(root
            .join("annotations")
            .join(format!("{cid}.pbtxt"))
            .is_file());
    }
}

#[test]
fn codex_import_projections_point_to_correct_utf8_bytes() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    let state = connection(&root.join("state_5.sqlite")).unwrap();
    state.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY,rollout_path TEXT,updated_at INTEGER,created_at INTEGER,
        source TEXT NOT NULL,model_provider TEXT NOT NULL,cwd TEXT NOT NULL,sandbox_policy TEXT NOT NULL,
        approval_mode TEXT NOT NULL,cli_version TEXT NOT NULL,originator TEXT NOT NULL,creator_user_id TEXT,creator_account_id TEXT,
        model TEXT,reasoning_effort TEXT,title TEXT,history_mode TEXT,thread_source TEXT,archived INTEGER,extra_required INTEGER NOT NULL);") .unwrap();
    state.execute("INSERT INTO threads(id,updated_at,source,model_provider,cwd,sandbox_policy,approval_mode,cli_version,originator,model,reasoning_effort,thread_source,archived,extra_required) VALUES ('template',1,'vscode','custom','/','{\"type\":\"read-only\"}','on-request','9.1','custom-desktop','fixture-model','low','user',0,7)",[]).unwrap();
    drop(state);
    let result = codex::import(&sample(), root, None, true, false).unwrap();
    let tid = result["cid"].as_str().unwrap();
    let state = readonly(&root.join("state_5.sqlite")).unwrap();
    let (rpath, model, extra): (String, String, i64) = state
        .query_row(
            "SELECT rollout_path,model,extra_required FROM threads WHERE id=?",
            [tid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(model, "fixture-model");
    assert_eq!(extra, 0);
    let bytes = fs::read(rpath).unwrap();
    let rows: Vec<Value> = bytes
        .split(|b| *b == b'\n')
        .filter(|b| !b.is_empty())
        .map(|b| serde_json::from_slice(b).unwrap())
        .collect();
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(row["ordinal"], i as u64);
    }
    assert_eq!(rows[0]["payload"]["history_mode"], "paginated");
    let c = readonly(&root.join("thread_history_1.sqlite")).unwrap();
    let (next,ordinal):(i64,i64)=c.query_row("SELECT next_rollout_byte_offset,next_rollout_ordinal FROM thread_history_projection_state WHERE thread_id=?",[tid],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert_eq!(next, bytes.len() as i64);
    assert_eq!(ordinal, rows.len() as i64);
    let mut st=c.prepare("SELECT rollout_byte_offset,rollout_end_byte_offset,rollout_ordinal,rollout_end_ordinal,status FROM thread_turns WHERE thread_id=? ORDER BY rollout_ordinal").unwrap();
    let turns = st
        .query_map([tid], |r| {
            Ok((
                r.get::<_, usize>(0)?,
                r.get::<_, usize>(1)?,
                r.get::<_, usize>(2)?,
                r.get::<_, usize>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].4, "failed");
    assert_eq!(turns[1].4, "completed");
    for (start, end, start_ord, end_ord, _) in turns {
        let slice = &bytes[start..end];
        let first =
            serde_json::from_slice::<Value>(slice.split(|b| *b == b'\n').next().unwrap()).unwrap();
        assert_eq!(first["ordinal"], start_ord as u64);
        assert_eq!(first["payload"]["type"], "task_started");
        let last: Value = serde_json::from_slice(
            slice
                .split(|b| *b == b'\n')
                .filter(|b| !b.is_empty())
                .last()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(last["ordinal"], end_ord as u64);
        assert_eq!(last["payload"]["type"], "task_complete");
    }
}

#[test]
fn claude_import_pairs_tools_and_links_desktop() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path().join("claude");
    let desktop = d.path().join("desktop");
    let result = claude::import(&sample(), &root, None, true, false, Some(&desktop)).unwrap();
    let sid = result["cid"].as_str().unwrap();
    let path = recent_files(&root.join("projects"), 2, "jsonl").remove(0);
    let lines = crate::storage::read_jsonl(&path).unwrap();
    let call = lines
        .iter()
        .find(|v| v["message"]["stop_reason"] == "tool_use")
        .unwrap();
    let tool = call["message"]["content"][0]["id"].as_str().unwrap();
    let result = lines
        .iter()
        .find(|v| v["message"]["content"][0]["tool_use_id"] == tool)
        .unwrap();
    assert_eq!(result["parentUuid"], call["uuid"]);
    assert_eq!(result["sourceToolAssistantUUID"], call["uuid"]);
    assert_eq!(
        result["message"]["content"][0]["content"][0]["text"],
        "Файл создан 🐱"
    );
    let compact = lines
        .iter()
        .find(|v| v["subtype"] == "compact_boundary")
        .unwrap();
    assert!(compact["parentUuid"].is_null());
    assert!(compact["logicalParentUuid"].is_string());
    let title: Value = serde_json::from_slice(
        &fs::read(path.parent().unwrap().join(sid).join("custom-title.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(title["customTitle"], "Проверка 🐱");
    let desktop_path = recent_files(&desktop, 1, "json").remove(0);
    let desktop: Value = serde_json::from_slice(&fs::read(desktop_path).unwrap()).unwrap();
    assert_eq!(desktop["cliSessionId"], sid);
    assert_eq!(desktop["completedTurns"], 2);
    assert_eq!(desktop["postTurnSummary"]["status_detail"], "Готово");
}

#[test]
fn opencode_import_ends_idle_and_keeps_template_model() {
    let d = tempfile::tempdir().unwrap();
    let db = d.path().join("opencode.db");
    let first = opencode::import(&sample(), &db, None, true).unwrap();
    let first_id = first["cid"].as_str().unwrap();
    let c = connection(&db).unwrap();
    c.execute(
        "UPDATE session_v2 SET slug='real-session',version='9.9',agent='plan',model=? WHERE id=?",
        params![
            "{\"id\":\"custom-model\",\"providerID\":\"custom-provider\"}",
            first_id
        ],
    )
    .unwrap();
    drop(c);
    let second = opencode::import(&sample(), &db, None, true).unwrap();
    let sid = second["cid"].as_str().unwrap();
    let c = readonly(&db).unwrap();
    let (version, agent, model): (String, String, String) = c
        .query_row(
            "SELECT version,agent,model FROM session_v2 WHERE id=?",
            [sid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(version, "9.9");
    assert_eq!(agent, "plan");
    assert_eq!(
        serde_json::from_str::<Value>(&model).unwrap()["id"],
        "custom-model"
    );
    let (last_type, max_seq): (String, i64) = c
        .query_row(
            "SELECT type,seq FROM session_message WHERE session_id=? ORDER BY seq DESC LIMIT 1",
            [sid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(last_type, "idle");
    assert_eq!(
        c.query_row(
            "SELECT seq FROM event_sequence WHERE aggregate_id=?",
            [sid],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        max_seq + 1
    );
    assert_eq!(
        c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
}

#[test]
fn filtering_keeps_only_original_supported_categories() {
    assert_eq!(
        filtered(&sample(), false)
            .unwrap()
            .iter()
            .map(|e| s(e, "kind"))
            .collect::<Vec<_>>(),
        ["user", "assistant", "compact", "user", "assistant"]
    );
}

#[test]
fn failed_imports_remove_only_their_own_new_session() {
    let d = tempfile::tempdir().unwrap();
    let ag = d.path().join("ag");
    let c = connection(&ag.join("conversation_summaries.db")).unwrap();
    c.execute_batch("CREATE TABLE conversation_summaries(conversation_id TEXT PRIMARY KEY,title TEXT);INSERT INTO conversation_summaries VALUES('existing','Keep me');").unwrap();
    drop(c);
    assert!(antigravity::import(&sample(), &ag, false, None, true).is_err());
    assert!(recent_files(&ag.join("conversations"), 1, "db").is_empty());
    assert!(recent_files(&ag.join("brain"), 6, "jsonl").is_empty());
    assert_eq!(
        readonly(&ag.join("conversation_summaries.db"))
            .unwrap()
            .query_row(
                "SELECT title FROM conversation_summaries WHERE conversation_id='existing'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "Keep me"
    );

    let codex = d.path().join("codex");
    fs::create_dir_all(&codex).unwrap();
    let previous = b"{\"id\":\"existing\",\"thread_name\":\"Keep me\"}";
    fs::write(codex.join("session_index.jsonl"), previous).unwrap();
    let c = connection(&codex.join("state_5.sqlite")).unwrap();
    c.execute_batch("CREATE TABLE threads(id TEXT PRIMARY KEY,title TEXT CHECK(title='existing'));INSERT INTO threads VALUES('existing','existing');").unwrap();
    drop(c);
    assert!(codex::import(&sample(), &codex, None, true, false).is_err());
    assert_eq!(
        fs::read(codex.join("session_index.jsonl")).unwrap(),
        previous
    );
    assert!(recent_files(&codex.join("sessions"), 6, "jsonl").is_empty());
    assert_eq!(
        readonly(&codex.join("state_5.sqlite"))
            .unwrap()
            .query_row("SELECT COUNT(*) FROM threads", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn cleanup_preserves_existing_workspace_and_concurrent_index_rows() {
    let d = tempfile::tempdir().unwrap();
    let workspace = d.path().join("existing");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join("keep"), b"keep").unwrap();
    let index = d.path().join("index.jsonl");
    let before = b"{\"id\":\"old\"}\n".to_vec();
    let written = b"{\"id\":\"old\"}\n{\"id\":\"owned\"}\n".to_vec();
    let mut guard = ImportGuard::default();
    guard.own(workspace.clone());
    guard.index(index.clone(), "id", "owned", Some(before), written);
    fs::write(
        &index,
        b"{\"id\":\"old\"}\n{\"id\":\"owned\"}\n{\"id\":\"concurrent\"}\n",
    )
    .unwrap();
    drop(guard);
    assert!(workspace.join("keep").is_file());
    assert_eq!(
        fs::read(&index).unwrap(),
        b"{\"id\":\"old\"}\n{\"id\":\"concurrent\"}\n"
    );
}

fn native_tool_sample(dir: &Path) -> Value {
    let dir_s = dir.to_string_lossy().into_owned();
    let main_cpp = dir
        .join("src")
        .join("main.cpp")
        .to_string_lossy()
        .into_owned();
    let menu_cpp = dir
        .join("src")
        .join("menu.cpp")
        .to_string_lossy()
        .into_owned();
    let notes_md = dir.join("notes.md").to_string_lossy().into_owned();
    json!({
        "meta": {
            "source_provider": "deepseek",
            "source_id": "native-tool-sample",
            "title": "Native Tools Check",
            "directory": dir_s
        },
        "entries": [
            {"kind":"user","text":"Inspect project, edit menu, run build.","ts":"2026-11-26T10:00:00Z"},
            {"kind":"tool","subtype":"thinking","tool":"thinking","text":"Check directory and read main.cpp first","ts":"2026-11-26T10:00:01Z"},
            {"kind":"tool","subtype":"tool_use","tool":"list_dir","text":serde_json::to_string(&json!({"path":dir_s})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:02Z"},
            {"kind":"tool","subtype":"tool_result","tool":"list_dir","text":"src/\nCMakeLists.txt","status":"ok","ts":"2026-11-26T10:00:03Z"},
            {"kind":"tool","subtype":"tool_use","tool":"read_file","text":serde_json::to_string(&json!({"path":main_cpp,"start_line":1,"end_line":40})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:04Z"},
            {"kind":"tool","subtype":"tool_result","tool":"read_file","text":"int main() { return 0; }","status":"ok","ts":"2026-11-26T10:00:05Z"},
            {"kind":"tool","subtype":"tool_use","tool":"edit_file","text":serde_json::to_string(&json!({"path":menu_cpp,"old_string":"bool open = false;","new_string":"bool open = true;"})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:06Z"},
            {"kind":"tool","subtype":"tool_result","tool":"edit_file","text":"Updated menu.cpp","status":"ok","ts":"2026-11-26T10:00:07Z"},
            {"kind":"tool","subtype":"tool_use","tool":"write_file","text":serde_json::to_string(&json!({"path":notes_md,"content":"# Notes\nReady\n"})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:08Z"},
            {"kind":"tool","subtype":"tool_result","tool":"write_file","text":"Created notes.md","status":"ok","ts":"2026-11-26T10:00:09Z"},
            {"kind":"tool","subtype":"tool_use","tool":"grep_search","text":serde_json::to_string(&json!({"pattern":"ImGui","path":dir_s})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:10Z"},
            {"kind":"tool","subtype":"tool_result","tool":"grep_search","text":"src/menu.cpp:1: // ImGui","status":"ok","ts":"2026-11-26T10:00:11Z"},
            {"kind":"tool","subtype":"tool_use","tool":"run_command","text":serde_json::to_string(&json!({"command":"cmake --build build"})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:12Z"},
            {"kind":"tool","subtype":"tool_result","tool":"run_command","text":"[100%] Built target app","status":"ok","ts":"2026-11-26T10:00:13Z"},
            {"kind":"tool","subtype":"tool_use","tool":"web_search","text":serde_json::to_string(&json!({"query":"imgui Docking"})).unwrap(),"status":"ok","ts":"2026-11-26T10:00:14Z"},
            {"kind":"tool","subtype":"tool_result","tool":"web_search","text":"ImGui docking branch docs","status":"ok","ts":"2026-11-26T10:00:15Z"},
            {"kind":"tool","subtype":"tool_use","tool":"custom_lookup","text":"{\"id\":42}","status":"ok","ts":"2026-11-26T10:00:16Z"},
            {"kind":"tool","subtype":"tool_result","tool":"custom_lookup","text":"entry #42","status":"ok","ts":"2026-11-26T10:00:17Z"},
            {"kind":"assistant","text":"Done building the project.","ts":"2026-11-26T10:00:18Z"}
        ]
    })
}

#[test]
fn native_tool_calls_format_properly_for_every_client() {
    let d = tempfile::tempdir().unwrap();
    let workspace = d.path().join("proj");
    fs::create_dir_all(&workspace).unwrap();
    let doc = native_tool_sample(&workspace);

    let cx_root = d.path().join("codex");
    let cx_res = codex::import(&doc, &cx_root, None, true, false).unwrap();
    let tid = cx_res["cid"].as_str().unwrap();
    let cx_rollout = recent_files(&cx_root.join("sessions"), 6, "jsonl").remove(0);
    let cx_rows = crate::storage::read_jsonl(&cx_rollout).unwrap();
    for row in &cx_rows {
        if row["type"] == "response_item" {
            let payload = &row["payload"];
            let ptype = payload["type"].as_str().unwrap_or("");
            let id = payload["id"].as_str().unwrap_or("");
            match ptype {
                "message" => assert!(id.starts_with("msg_"), "message id prefix: {id}"),
                "reasoning" => assert!(id.starts_with("rs_"), "reasoning id prefix: {id}"),
                "function_call" => {
                    assert!(id.starts_with("fc_"), "function_call id prefix: {id}");
                    assert!(
                        payload["call_id"]
                            .as_str()
                            .is_some_and(|c| c.starts_with("call_")),
                        "function_call call_id prefix"
                    );
                }
                "function_call_output" => {
                    assert!(
                        id.starts_with("fco_"),
                        "function_call_output id prefix: {id}"
                    );
                    assert!(
                        payload["call_id"]
                            .as_str()
                            .is_some_and(|c| c.starts_with("call_")),
                        "function_call_output call_id prefix"
                    );
                }
                "custom_tool_call" => {
                    assert!(id.starts_with("ctc_"), "custom_tool_call id prefix: {id}");
                    assert!(
                        payload["call_id"]
                            .as_str()
                            .is_some_and(|c| c.starts_with("call_")),
                        "custom_tool_call call_id prefix"
                    );
                }
                "custom_tool_call_output" => {
                    assert!(
                        id.starts_with("ctco_"),
                        "custom_tool_call_output id prefix: {id}"
                    );
                    assert!(
                        payload["call_id"]
                            .as_str()
                            .is_some_and(|c| c.starts_with("call_")),
                        "custom_tool_call_output call_id prefix"
                    );
                }
                _ => {}
            }
        }
        let txt_msg = row["payload"]["item"]["content"][0]["text"]
            .as_str()
            .or_else(|| row["payload"]["content"][0]["text"].as_str())
            .unwrap_or("");
        assert!(
            !txt_msg.starts_with("[edit_file]")
                && !txt_msg.starts_with("[read_file]")
                && !txt_msg.starts_with("[run_command]")
                && !txt_msg.starts_with("[write_file]"),
            "Codex must not emit fake [tool] text: {txt_msg}"
        );
    }
    assert!(cx_rows.iter().any(|r| {
        r["type"] == "response_item"
            && r["payload"]["type"] == "custom_tool_call"
            && r["payload"]["name"] == "apply_patch"
            && r["payload"]["input"]
                .as_str()
                .is_some_and(|s| s.contains("*** Update File:") && s.contains("+bool open = true;"))
    }));
    assert!(cx_rows.iter().any(|r| {
        r["type"] == "response_item"
            && r["payload"]["type"] == "function_call"
            && r["payload"]["name"] == "exec_command"
            && r["payload"]["arguments"]
                .as_str()
                .is_some_and(|s| s.contains("cmake --build build"))
    }));
    assert!(cx_rows
        .iter()
        .any(|r| r["type"] == "response_item" && r["payload"]["type"] == "reasoning"));
    let cx_db = readonly(&cx_root.join("thread_history_1.sqlite")).unwrap();
    let mut item_types: Vec<(String, String)> = cx_db
        .prepare("SELECT item_type, item_json FROM thread_items WHERE thread_id=? ORDER BY rollout_ordinal")
        .unwrap()
        .query_map([tid], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(item_types.iter().any(|(t, j)| {
        t == "fileChange" && j.contains("\"type\":\"update\"") && j.contains("bool open = true;")
    }));
    assert!(item_types
        .iter()
        .any(|(t, j)| t == "fileChange" && j.contains("\"type\":\"add\"")));
    assert!(item_types.iter().any(|(t, j)| {
        t == "commandExecution" && j.contains("\"type\":\"read\"") && j.contains("main.cpp")
    }));
    assert!(item_types
        .iter()
        .any(|(t, j)| t == "commandExecution" && j.contains("cmake --build build")));
    assert!(item_types.iter().any(|(t, _)| t == "webSearch"));
    assert!(item_types
        .iter()
        .any(|(t, j)| t == "mcpToolCall" && j.contains("\"custom_lookup\"")));
    assert!(!item_types.is_empty());
    item_types.clear();

    let cl_root = d.path().join("claude");
    claude::import(&doc, &cl_root, None, true, false, None).unwrap();
    let cl_file = recent_files(&cl_root.join("projects"), 2, "jsonl").remove(0);
    let cl_rows = crate::storage::read_jsonl(&cl_file).unwrap();
    let cl_tools: Vec<(&str, &Value)> = cl_rows
        .iter()
        .filter(|r| r["type"] == "assistant" && r["message"]["stop_reason"] == "tool_use")
        .map(|r| {
            let b = &r["message"]["content"][0];
            (b["name"].as_str().unwrap(), &b["input"])
        })
        .collect();
    assert_eq!(cl_tools[0].0, "Glob");
    assert_eq!(cl_tools[1].0, "Read");
    assert!(cl_tools[1].1["file_path"]
        .as_str()
        .unwrap()
        .ends_with("main.cpp"));
    assert_eq!(cl_tools[2].0, "Edit");
    assert_eq!(cl_tools[2].1["old_string"], "bool open = false;");
    assert_eq!(cl_tools[2].1["new_string"], "bool open = true;");
    assert_eq!(cl_tools[3].0, "Write");
    assert_eq!(cl_tools[4].0, "Grep");
    assert_eq!(cl_tools[5].0, "Bash");
    assert_eq!(cl_tools[5].1["command"], "cmake --build build");
    assert_eq!(cl_tools[6].0, "WebSearch");
    assert_eq!(cl_tools[7].0, "custom_lookup");

    let ag_root = d.path().join("antigravity");
    let ag_res = antigravity::import(&doc, &ag_root, false, None, true).unwrap();
    let ag_cid = ag_res["cid"].as_str().unwrap();
    let ag_db = readonly(&ag_root.join("conversations").join(format!("{ag_cid}.db"))).unwrap();
    let planner_calls: Vec<(String, String, Option<String>)> = ag_db
        .prepare("SELECT idx, step_payload FROM steps WHERE step_type=15 ORDER BY idx")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?)))
        .unwrap()
        .filter_map(|row| {
            let (_, payload) = row.ok()?;
            let name = crate::pbwire::string_at(&payload, &[20, 7, 2])?;
            let args = crate::pbwire::string_at(&payload, &[20, 7, 3])?;
            let th = crate::pbwire::string_at(&payload, &[20, 3]);
            Some((name, args, th))
        })
        .collect();
    assert_eq!(planner_calls.len(), 8);
    assert_eq!(planner_calls[0].0, "list_dir");
    assert_eq!(
        planner_calls[0].2.as_deref(),
        Some("Check directory and read main.cpp first")
    );
    assert_eq!(planner_calls[1].0, "read_file");
    assert_eq!(planner_calls[2].0, "edit_file");
    assert!(planner_calls[2]
        .1
        .contains("\"Target\":\"bool open = false;\""));
    assert_eq!(planner_calls[5].0, "run_command");
    assert!(planner_calls[5]
        .1
        .contains("\"CommandLine\":\"cmake --build build\""));
    let gen_out: Vec<String> = ag_db
        .prepare("SELECT step_payload FROM steps WHERE step_type=132 ORDER BY idx")
        .unwrap()
        .query_map([], |r| r.get::<_, Vec<u8>>(0))
        .unwrap()
        .filter_map(|row| crate::pbwire::string_at(&row.ok()?, &[140, 2, 1]))
        .collect();
    assert_eq!(gen_out.len(), 8);
    assert_eq!(gen_out[2], "Updated menu.cpp");
    let ag_hist = ag_root
        .join("brain")
        .join(ag_cid)
        .join(".system_generated/logs/transcript_full.jsonl");
    let ag_lines = crate::storage::read_jsonl(&ag_hist).unwrap();
    assert_eq!(ag_lines[1]["type"], "PLANNER_RESPONSE");
    assert_eq!(ag_lines[1]["tool_calls"][0]["name"], "list_dir");
    assert_eq!(ag_lines[2]["type"], "GENERIC");
    assert_eq!(ag_lines[2]["content"], "src/\nCMakeLists.txt");

    let oc_db = d.path().join("opencode.db");
    let oc_res = opencode::import(&doc, &oc_db, None, true).unwrap();
    let oc_sid = oc_res["cid"].as_str().unwrap();
    let oc = readonly(&oc_db).unwrap();
    let msg_rows: Vec<String> = oc
        .prepare(
            "SELECT data FROM session_message WHERE session_id=? AND type='assistant' ORDER BY seq",
        )
        .unwrap()
        .query_map([oc_sid], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let tool_parts: Vec<Value> = msg_rows
        .iter()
        .flat_map(|raw| {
            serde_json::from_str::<Value>(raw).unwrap()["content"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter(|p| p["type"] == "tool")
        .collect();
    assert_eq!(tool_parts.len(), 8);
    assert_eq!(tool_parts[0]["name"], "glob");
    assert_eq!(tool_parts[1]["name"], "read");
    assert_eq!(tool_parts[2]["name"], "edit");
    assert_eq!(
        tool_parts[2]["state"]["input"]["oldString"],
        "bool open = false;"
    );
    assert_eq!(
        tool_parts[2]["state"]["input"]["newString"],
        "bool open = true;"
    );
    assert_eq!(
        tool_parts[2]["state"]["content"][0]["text"],
        "Updated menu.cpp"
    );
    assert_eq!(tool_parts[3]["name"], "write");
    assert_eq!(tool_parts[4]["name"], "grep");
    assert_eq!(tool_parts[5]["name"], "shell");
    assert_eq!(
        tool_parts[5]["state"]["input"]["command"],
        "cmake --build build"
    );
    assert_eq!(tool_parts[6]["name"], "websearch");
    assert_eq!(tool_parts[7]["name"], "custom_lookup");
}
