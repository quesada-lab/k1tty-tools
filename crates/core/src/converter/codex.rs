use super::*;
use rusqlite::{
    params, params_from_iter,
    types::{Value as SqlValue, ValueRef},
};
use std::io::{BufRead, BufReader};

fn sql_json(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => json!(n),
        ValueRef::Real(n) => json!(n),
        ValueRef::Text(s) => json!(String::from_utf8_lossy(s)),
        ValueRef::Blob(s) => json!(String::from_utf8_lossy(s)),
    }
}
fn json_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(v) => SqlValue::Integer(i64::from(*v)),
        Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                SqlValue::Integer(i)
            } else {
                SqlValue::Real(v.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => SqlValue::Text(s.clone()),
        v => SqlValue::Text(v.to_string()),
    }
}
fn template(home: &Path) -> Value {
    let mut tpl = json!({"account_scoped":false,"creator_user_id":null,"creator_account_id":null,
        "originator":"Codex Desktop","cli_version":"0.169.0-alpha.11.1","source":"vscode","model_provider":"openai",
        "sandbox_policy":"{\"type\":\"disabled\"}","approval_mode":"never","model":"gpt-6.1-sol","reasoning_effort":"high",
        "cwd":home_dir(),"runtime_workspace_roots":[],"base_instructions":null,"turn_context":null,"rollout_path":null});
    let mut candidates = Vec::new();
    let _ = (|| -> Result<()> {
        let c = readonly(&home.join("state_5.sqlite"))?;
        let mut stmt = c.prepare("PRAGMA table_info(threads)")?;
        let cols: Vec<String> = stmt
            .query_map([], |r| r.get(1))?
            .collect::<std::result::Result<_, _>>()?;
        let want: Vec<&str> = [
            "source",
            "model_provider",
            "cwd",
            "sandbox_policy",
            "approval_mode",
            "cli_version",
            "originator",
            "creator_user_id",
            "creator_account_id",
            "model",
            "reasoning_effort",
            "rollout_path",
        ]
        .into_iter()
        .filter(|n| cols.iter().any(|c| c == n))
        .collect();
        if want.is_empty() {
            return Ok(());
        }
        let mut clauses = Vec::new();
        if cols.iter().any(|c| c == "thread_source") {
            if cols.iter().any(|c| c == "archived") {
                clauses.push("WHERE thread_source='user' AND archived=0");
            }
            clauses.push("WHERE thread_source='user'");
        }
        clauses.push("WHERE source NOT LIKE '{%'");
        clauses.push("");
        for clause in clauses {
            let mut stmt = c.prepare(&format!(
                "SELECT {} FROM threads {clause} ORDER BY updated_at DESC LIMIT 12",
                want.join(",")
            ))?;
            let rows = stmt
                .query_map([], |row| {
                    let mut vals = Vec::new();
                    for i in 0..want.len() {
                        vals.push(sql_json(row.get_ref(i)?));
                    }
                    Ok(vals)
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            if rows.is_empty() {
                continue;
            }
            for (key, val) in want.iter().zip(&rows[0]) {
                if !val.is_null() && val != &json!("") {
                    tpl[*key] = if matches!(*key, "cwd" | "rollout_path") {
                        json!(strip_long(val.as_str().unwrap_or("")))
                    } else {
                        val.clone()
                    };
                }
            }
            tpl["account_scoped"] = json!(
                !s(&tpl, "creator_user_id").is_empty() || !s(&tpl, "creator_account_id").is_empty()
            );
            if let Some(i) = want.iter().position(|n| *n == "rollout_path") {
                for row in rows {
                    if let Some(p) = row[i].as_str().filter(|s| !s.is_empty()) {
                        candidates.push(strip_long(p));
                    }
                }
            }
            break;
        }
        Ok(())
    })();
    for path in candidates {
        let Ok(f) = fs::File::open(path) else {
            continue;
        };
        for line in BufReader::new(f).split(b'\n').take(35) {
            let Ok(raw) = line else { break };
            let Ok(v) = serde_json::from_slice::<Value>(&raw) else {
                break;
            };
            let p = &v["payload"];
            if v["type"] == "session_meta" {
                if tpl["base_instructions"].is_null() && p["base_instructions"].is_object() {
                    tpl["base_instructions"] = p["base_instructions"].clone();
                }
                if tpl["runtime_workspace_roots"]
                    .as_array()
                    .is_none_or(Vec::is_empty)
                {
                    if let Some(roots) = p["runtime_workspace_roots"].as_array() {
                        tpl["runtime_workspace_roots"] = json!(roots
                            .iter()
                            .filter_map(Value::as_str)
                            .filter(|s| !s.is_empty())
                            .map(strip_long)
                            .collect::<Vec<_>>());
                    }
                }
            } else if v["type"] == "turn_context" && tpl["turn_context"].is_null() {
                tpl["turn_context"] = p.clone();
            }
            if !tpl["base_instructions"].is_null() && !tpl["turn_context"].is_null() {
                break;
            }
        }
        if !tpl["base_instructions"].is_null() && !tpl["turn_context"].is_null() {
            break;
        }
    }
    tpl
}
fn projection_schema(c: &Connection) -> Result<()> {
    c.execute_batch("CREATE TABLE IF NOT EXISTS thread_turns (thread_id TEXT NOT NULL,turn_id TEXT NOT NULL,rollout_ordinal INTEGER NOT NULL,
        status TEXT NOT NULL,error_json TEXT,started_at INTEGER,completed_at INTEGER,duration_ms INTEGER,first_user_item_id TEXT,
        final_agent_item_id TEXT,rollout_byte_offset INTEGER,rollout_end_ordinal INTEGER,rollout_end_byte_offset INTEGER,root_turn_id TEXT,
        PRIMARY KEY(thread_id,turn_id));
        CREATE TABLE IF NOT EXISTS thread_items (thread_id TEXT NOT NULL,turn_id TEXT NOT NULL,item_id TEXT NOT NULL,
        rollout_ordinal INTEGER NOT NULL,created_at_ms INTEGER NOT NULL,item_json TEXT NOT NULL,item_type TEXT NOT NULL DEFAULT '',
        updated_at_ordinal INTEGER NOT NULL DEFAULT 0,started_at_ms INTEGER,completed_at_ms INTEGER,PRIMARY KEY(thread_id,turn_id,item_id));
        CREATE TABLE IF NOT EXISTS thread_history_projection_state (thread_id TEXT PRIMARY KEY,next_rollout_byte_offset INTEGER NOT NULL,next_rollout_ordinal INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS thread_realtime_items (thread_id TEXT NOT NULL,item_id TEXT NOT NULL,rollout_ordinal INTEGER NOT NULL,
        created_at_ms INTEGER NOT NULL,item_type TEXT NOT NULL,item_json TEXT NOT NULL,PRIMARY KEY(thread_id,item_id));")?;
    Ok(())
}
fn update_global(home: &Path, tid: &str, cwd: &str, roots: &[String]) -> Result<()> {
    let path = home.join(".codex-global-state.json");
    if !path.is_file() {
        return Ok(());
    }
    let mut data: Value = serde_json::from_slice(&fs::read(&path)?)?;
    if !data.is_object() {
        return Ok(());
    }
    let norm = |s: &str| {
        let s = lexical_normpath(&strip_long(s));
        if cfg!(windows) {
            s.to_lowercase()
        } else {
            s
        }
    };
    let target = norm(cwd);
    let mut project = None;
    if let Some(projects) = data["local-projects"].as_object() {
        for (pid, info) in projects {
            if info["rootPaths"].as_array().is_some_and(|rs| {
                rs.iter()
                    .filter_map(Value::as_str)
                    .any(|r| norm(r) == target)
            }) {
                project = Some(pid.clone());
                break;
            }
        }
    }
    if let Some(pid) = &project {
        if !data["thread-project-assignments"].is_object() {
            data["thread-project-assignments"] = json!({});
        }
        data["thread-project-assignments"][tid] = json!({"projectKind":"local","projectId":pid});
    } else {
        if !data["projectless-thread-ids"].is_array() {
            data["projectless-thread-ids"] = json!([]);
        }
        let ids = data["projectless-thread-ids"].as_array_mut().unwrap();
        if !ids.iter().any(|v| v == tid) {
            ids.push(json!(tid));
        }
    }
    let hint = roots.first().cloned().unwrap_or_else(|| strip_long(cwd));
    let outputs = Path::new(&strip_long(cwd)).join("outputs");
    for (key, val) in data.as_object_mut().unwrap() {
        if key.starts_with("electron-owner-value:thread-workspace-root-hints:") && val.is_object() {
            val[tid] = json!(hint);
        } else if project.is_none()
            && key.starts_with("electron-owner-value:thread-projectless-output-directories:")
            && val.is_object()
        {
            val[tid] = json!(outputs);
        }
    }
    write_json(&path, &data)
}
#[derive(Default)]
struct Rollout {
    bytes: Vec<u8>,
    ordinal: i64,
}
impl Rollout {
    fn emit(&mut self, ts: &str, kind: &str, payload: Value) -> Result<(i64, i64, i64)> {
        let ord = self.ordinal;
        let start = self.bytes.len() as i64;
        serde_json::to_writer(
            &mut self.bytes,
            &json!({"timestamp":ts,"ordinal":ord,"type":kind,"payload":payload}),
        )?;
        self.bytes.push(b'\n');
        self.ordinal += 1;
        Ok((ord, start, self.bytes.len() as i64))
    }
}
struct ProjItem {
    turn: String,
    id: String,
    ordinal: i64,
    ms: i64,
    item: Value,
    kind: String,
}
fn windows_cwd(path: &str) -> String {
    let path = strip_long(path);
    if path.starts_with(r"\\") {
        path
    } else {
        format!(r"\\?\{path}")
    }
}

struct ProjTurn {
    turn: String,
    start: i64,
    status: String,
    error: Option<String>,
    started: i64,
    completed: i64,
    duration: i64,
    user: Option<String>,
    agent: Option<String>,
    offset: i64,
    end: i64,
    end_offset: i64,
}

pub(super) fn import(
    doc: &Value,
    home: &Path,
    title: Option<&str>,
    include_tools: bool,
    default_home: bool,
) -> Result<Value> {
    let entries = filtered(doc, include_tools)?;
    let title = title_for(doc, &entries, title);
    let tpl = template(home);
    let mut guard = ImportGuard::default();
    let now = now_sec();
    let now_dt = Utc::now();
    let tid = uuid7((now * 1000.0) as i64);
    let mut roots: Vec<String> = tpl["runtime_workspace_roots"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let meta_dir = strip_long(&s(&doc["meta"], "directory"));
    let cwd = if !meta_dir.is_empty() && !ephemeral(&meta_dir) && Path::new(&meta_dir).is_dir() {
        lexical_normpath(&meta_dir)
    } else if default_home && roots.first().is_some_and(|r| Path::new(r).is_dir()) {
        let path = Path::new(&roots[0])
            .join(now_dt.format("%Y-%m-%d").to_string())
            .join(format!("imported-{}", &tid[..8]));
        guard.own(path.clone());
        fs::create_dir_all(path.join("outputs"))?;
        path.to_string_lossy().into_owned()
    } else if !default_home {
        let path = home
            .join("workspaces")
            .join(format!("imported-{}", &tid[..8]));
        guard.own(path.clone());
        fs::create_dir_all(path.join("outputs"))?;
        path.to_string_lossy().into_owned()
    } else {
        let tc = strip_long(&s(&tpl, "cwd"));
        if !tc.is_empty() && Path::new(&tc).is_dir() {
            tc
        } else {
            home_dir().to_string_lossy().into_owned()
        }
    };
    if roots.is_empty() {
        roots.push(
            Path::new(&cwd)
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(&cwd))
                .to_string_lossy()
                .into_owned(),
        );
    }
    let day = home
        .join("sessions")
        .join(now_dt.format("%Y/%m/%d").to_string());
    fs::create_dir_all(&day)?;
    let path = day.join(format!(
        "rollout-{}-{tid}.jsonl",
        now_dt.format("%Y-%m-%dT%H-%M-%S")
    ));
    guard.own(path.clone());
    let times = timestamps(&entries, now);
    let first = times[0];
    let last = times[times.len() - 1];
    let first_iso = iso(first);
    let mut rollout = Rollout::default();
    let mut sm = json!({"account_scoped":tpl["account_scoped"],"session_id":tid,"id":tid,"timestamp":first_iso,
        "cwd":cwd,"runtime_workspace_roots":roots,"originator":tpl["originator"],"cli_version":tpl["cli_version"],
        "source":tpl["source"],"thread_source":"user","model_provider":tpl["model_provider"],"history_mode":"paginated",
        "context_window":{"window_id":uuid7((first*1000.0) as i64)}});
    for key in ["creator_user_id", "creator_account_id", "base_instructions"] {
        if !tpl[key].is_null() && tpl[key] != json!("") && tpl[key] != json!({}) {
            sm[key] = tpl[key].clone();
        }
    }
    rollout.emit(&first_iso, "session_meta", sm)?;
    let model = nonempty(s(&tpl, "model")).unwrap_or_else(|| "gpt-6.1-sol".into());
    let effort = nonempty(s(&tpl, "reasoning_effort")).unwrap_or_else(|| "high".into());
    let mut turns: Vec<Vec<usize>> = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        if e["kind"] == "user" || turns.is_empty() {
            turns.push(vec![i]);
        } else {
            turns.last_mut().unwrap().push(i);
        }
    }
    let mut proj_turns = Vec::new();
    let mut proj_items = Vec::new();
    let mut first_user = String::new();
    for indices in turns {
        let start_sec = times[indices[0]];
        let end_sec = times[*indices.last().unwrap()];
        let start_ms = (start_sec * 1000.0) as i64;
        let end_ms = ((end_sec * 1000.0) as i64).max(start_ms + 1);
        let start_iso = iso(start_sec);
        let turn = uuid7(start_ms);
        let (start_ord,start_off,_)=rollout.emit(&start_iso,"event_msg",json!({"type":"task_started",
            "turn_attribution":{"turn_id":turn,"turn_trigger":"composer","parent_turn_id":null,"initiating_agent_path":null,"root_turn_id":turn},
            "turn_id":turn,"root_turn_id":turn,"started_at":start_sec as i64,"model_context_window":475000,"collaboration_mode_kind":"default"}))?;
        let mut tc = if tpl["turn_context"].is_object() {
            tpl["turn_context"].clone()
        } else {
            json!({
            "disabled_plugin_ids":[],"approval_policy":tpl["approval_mode"],"approvals_reviewer":"user","permission_profile":{"type":"disabled"},
            "active_permission_profile":{"id":":danger-full-access"},"model":model,"collaboration_mode":{"mode":"default","settings":{"model":model,"reasoning_effort":effort}},
            "multi_agent_version":"v1","realtime_active":false,"effort":effort,"summary":"none","sandbox_policy":{"type":"danger-full-access"}})
        };
        tc["turn_id"] = json!(turn);
        tc["root_turn_id"] = json!(turn);
        tc["cwd"] = json!(cwd);
        rollout.emit(&start_iso, "turn_context", tc)?;
        let last_asst = indices
            .iter()
            .rposition(|i| entries[*i]["kind"] == "assistant");
        let mut user_id = None;
        let mut agent_id = None;
        let mut agent_text = String::new();
        let mut turn_error: Option<Value> = None;
        let mut pos = 0usize;
        while pos < indices.len() {
            let i = indices[pos];
            let entry = &entries[i];
            let sec = times[i];
            let ms = (sec * 1000.0) as i64;
            let ts = iso(sec);
            let kind = nonempty(s(entry, "kind")).unwrap_or_else(|| "assistant".into());
            let subtype = s(entry, "subtype");
            let text = s(entry, "text");
            if kind == "user" {
                pos += 1;
                if first_user.is_empty() {
                    first_user = shorten(&text, 400);
                }
                let uid = uuid7(ms);
                let client = uuid();
                if user_id.is_none() {
                    user_id = Some(uid.clone());
                }
                rollout.emit(&ts,"response_item",json!({"type":"message","id":format!("msg_{}",uuid7(ms)),"role":"user","content":[{"type":"input_text","text":text}],
                    "internal_chat_message_metadata_passthrough":{"turn_id":turn,"create_time":sec,"content_item_kinds":["user.text"],"content_item_metadata":[{"provenance":{"type":"user"}}]}}))?;
                let (ord,_,_)=rollout.emit(&ts,"event_msg",json!({"type":"item_completed","thread_id":tid,"turn_id":turn,
                    "item":{"type":"UserMessage","id":uid,"client_id":client,"content":[{"type":"text","text":text,"text_elements":[]}]},"started_at_ms":ms,"completed_at_ms":ms}))?;
                proj_items.push(ProjItem{turn:turn.clone(),id:uid.clone(),ordinal:ord,ms,item:json!({"type":"userMessage","id":uid,"clientId":client,"content":[{"type":"text","text":text,"text_elements":[]}]}),kind:"userMessage".into()});
            } else if kind == "assistant" {
                let phase = if last_asst == Some(pos) {
                    "final_answer"
                } else {
                    "commentary"
                };
                pos += 1;
                let mid = format!("msg_{}{}", uuid_hex(), &uuid_hex()[..16]);
                agent_id = Some(mid.clone());
                agent_text = text.clone();
                let (ord,_,_)=rollout.emit(&ts,"event_msg",json!({"type":"item_completed","thread_id":tid,"turn_id":turn,
                    "item":{"type":"AgentMessage","id":mid,"content":[{"type":"Text","text":text}],"phase":phase},
                    "content_token_count":(text.chars().count()/4).max(1),"started_at_ms":ms,"completed_at_ms":ms}))?;
                let payload = json!({"type":"message","status":"completed","id":mid,"role":"assistant",
                    "content":[{"type":"output_text","text":text,"annotations":[],"logprobs":[]}],"phase":phase,
                    "internal_chat_message_metadata_passthrough":{"turn_id":turn,"create_time":sec,"content_item_kinds":["unknown"]}});
                rollout.emit(&ts, "response_item", payload)?;
                proj_items.push(ProjItem{turn:turn.clone(),id:mid.clone(),ordinal:ord,ms,item:json!({"type":"agentMessage","moderation":null,"id":mid,"text":text,"phase":phase,"memoryCitation":null,"delivery":null,"questions":null}),kind:"agentMessage".into()});
            } else if kind == "tool" && subtype == "thinking" {
                pos += 1;
                if text.trim().is_empty() {
                    continue;
                }
                let rid = format!("rs_{}", uuid_hex());
                rollout.emit(
                    &ts,
                    "response_item",
                    json!({"type":"reasoning","id":rid,"summary":[{"type":"summary_text","text":text}]}),
                )?;
                let (ord,_,_)=rollout.emit(&ts,"event_msg",json!({"type":"item_completed","thread_id":tid,"turn_id":turn,
                    "item":{"type":"Reasoning","id":rid,"summary_text":[text],"raw_content":[]},"started_at_ms":ms,"completed_at_ms":ms}))?;
                proj_items.push(ProjItem {
                    turn: turn.clone(),
                    id: rid.clone(),
                    ordinal: ord,
                    ms,
                    item: json!({"type":"reasoning","id":rid,"summary":[text],"content":[]}),
                    kind: "reasoning".into(),
                });
            } else if kind == "tool" {
                let mut cursor = i;
                let inv = tools::consume_tool_pair(&entries, &times, &mut cursor);
                pos += cursor - i;
                let call_sec = inv.ts_call;
                let res_sec = inv.ts_result.max(call_sec);
                let call_ts = iso(call_sec);
                let res_ts = iso(res_sec);
                let c_ms = (call_sec * 1000.0) as i64;
                let r_ms = ((res_sec * 1000.0) as i64).max(c_ms + 50);
                let dur_ms = (r_ms - c_ms).max(1);
                let status_str = if inv.is_error { "failed" } else { "completed" };
                let call_id = format!("call_{}", &uuid_hex()[..24]);
                let fc_id = format!("fc_{}", uuid_hex());
                let fco_id = format!("fco_{}", uuid_hex());
                let ctc_id = format!("ctc_{}", uuid_hex());
                let ctco_id = format!("ctco_{}", uuid_hex());
                let item_id = format!("exec-{}", uuid());
                match inv.canonical {
                    tools::CanonicalTool::Write { path, content } => {
                        let patch = tools::build_apply_patch(&path, "", &content, true, false);
                        let out_str = if inv.result_text.is_empty() {
                            format!("Success. Updated the following files:\nA {path}\n")
                        } else {
                            inv.result_text.clone()
                        };
                        rollout.emit(
                            &call_ts,
                            "response_item",
                            json!({"type":"custom_tool_call","id":ctc_id,"status":"completed","call_id":call_id,"name":"apply_patch","input":patch}),
                        )?;
                        let mut changes = serde_json::Map::new();
                        changes.insert(path.clone(), json!({"type":"add","content":content}));
                        let (ord, _, _) = rollout.emit(
                            &res_ts,
                            "event_msg",
                            json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"FileChange","id":item_id,"changes":changes,"status":status_str,"stdout":out_str,"stderr":if inv.is_error{&out_str}else{""}},"started_at_ms":c_ms,"completed_at_ms":r_ms}),
                        )?;
                        rollout.emit(
                            &res_ts,
                            "response_item",
                            json!({"type":"custom_tool_call_output","id":ctco_id,"call_id":call_id,"output":out_str}),
                        )?;
                        proj_items.push(ProjItem {
                            turn: turn.clone(),
                            id: item_id.clone(),
                            ordinal: ord,
                            ms: c_ms,
                            item: json!({"type":"fileChange","id":item_id,"changes":[{"path":path,"kind":{"type":"add"},"diff":content}],"status":status_str}),
                            kind: "fileChange".into(),
                        });
                    }
                    tools::CanonicalTool::Delete { path } => {
                        let patch = tools::build_apply_patch(&path, "", "", false, true);
                        let out_str = if inv.result_text.is_empty() {
                            format!("Success. Updated the following files:\nD {path}\n")
                        } else {
                            inv.result_text.clone()
                        };
                        rollout.emit(
                            &call_ts,
                            "response_item",
                            json!({"type":"custom_tool_call","id":ctc_id,"status":"completed","call_id":call_id,"name":"apply_patch","input":patch}),
                        )?;
                        let mut changes = serde_json::Map::new();
                        changes.insert(path.clone(), json!({"type":"delete","content":""}));
                        let (ord, _, _) = rollout.emit(
                            &res_ts,
                            "event_msg",
                            json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"FileChange","id":item_id,"changes":changes,"status":status_str,"stdout":out_str,"stderr":if inv.is_error{&out_str}else{""}},"started_at_ms":c_ms,"completed_at_ms":r_ms}),
                        )?;
                        rollout.emit(
                            &res_ts,
                            "response_item",
                            json!({"type":"custom_tool_call_output","id":ctco_id,"call_id":call_id,"output":out_str}),
                        )?;
                        proj_items.push(ProjItem {
                            turn: turn.clone(),
                            id: item_id.clone(),
                            ordinal: ord,
                            ms: c_ms,
                            item: json!({"type":"fileChange","id":item_id,"changes":[{"path":path,"kind":{"type":"delete"},"diff":""}],"status":status_str}),
                            kind: "fileChange".into(),
                        });
                    }
                    tools::CanonicalTool::Edit {
                        path,
                        old_string,
                        new_string,
                        diff,
                        content,
                    } => {
                        let replacement = if new_string.is_empty() {
                            content.unwrap_or_default()
                        } else {
                            new_string
                        };
                        let udiff = diff.unwrap_or_else(|| {
                            tools::build_unified_diff(&old_string, &replacement)
                        });
                        let patch = tools::build_apply_patch(
                            &path,
                            &old_string,
                            &replacement,
                            false,
                            false,
                        );
                        let out_str = if inv.result_text.is_empty() {
                            format!("Success. Updated the following files:\nM {path}\n")
                        } else {
                            inv.result_text.clone()
                        };
                        rollout.emit(
                            &call_ts,
                            "response_item",
                            json!({"type":"custom_tool_call","id":ctc_id,"status":"completed","call_id":call_id,"name":"apply_patch","input":patch}),
                        )?;
                        let mut changes = serde_json::Map::new();
                        changes.insert(
                            path.clone(),
                            json!({"type":"update","unified_diff":udiff,"move_path":null}),
                        );
                        let (ord, _, _) = rollout.emit(
                            &res_ts,
                            "event_msg",
                            json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"FileChange","id":item_id,"changes":changes,"status":status_str,"stdout":out_str,"stderr":if inv.is_error{&out_str}else{""}},"started_at_ms":c_ms,"completed_at_ms":r_ms}),
                        )?;
                        rollout.emit(
                            &res_ts,
                            "response_item",
                            json!({"type":"custom_tool_call_output","id":ctco_id,"call_id":call_id,"output":out_str}),
                        )?;
                        proj_items.push(ProjItem {
                            turn: turn.clone(),
                            id: item_id.clone(),
                            ordinal: ord,
                            ms: c_ms,
                            item: json!({"type":"fileChange","id":item_id,"changes":[{"path":path,"kind":{"type":"update","move_path":null},"diff":udiff}],"status":status_str}),
                            kind: "fileChange".into(),
                        });
                    }
                    tools::CanonicalTool::Shell { .. }
                    | tools::CanonicalTool::Read { .. }
                    | tools::CanonicalTool::ListDir { .. }
                    | tools::CanonicalTool::Grep { .. } => {
                        let (cmd_str, parsed_cmd, command_actions) = match inv.canonical {
                            tools::CanonicalTool::Read {
                                path,
                                start_line,
                                end_line,
                            } => {
                                let qpath = tools::shell_quote(&path);
                                let cmd = if let (Some(s), Some(e)) = (start_line, end_line) {
                                    format!("sed -n '{s},{e}p' {qpath}")
                                } else {
                                    format!("cat {qpath}")
                                };
                                let bname = tools::basename(&path);
                                (
                                    cmd.clone(),
                                    json!([{"type":"read","cmd":cmd,"name":bname,"path":path}]),
                                    json!([{"type":"read","command":cmd,"name":bname,"path":path}]),
                                )
                            }
                            tools::CanonicalTool::Shell { command, .. } => (
                                command.clone(),
                                json!([{"type":"unknown","cmd":command}]),
                                json!([{"type":"unknown","command":command}]),
                            ),
                            tools::CanonicalTool::ListDir { path, pattern } => {
                                let qpath = tools::shell_quote(&path);
                                let cmd = if let Some(pat) =
                                    pattern.as_deref().filter(|s| !s.is_empty())
                                {
                                    format!("rg --files -g {} {qpath}", tools::shell_quote(pat))
                                } else {
                                    format!("ls -la {qpath}")
                                };
                                (
                                    cmd.clone(),
                                    json!([{"type":"unknown","cmd":cmd}]),
                                    json!([{"type":"unknown","command":cmd}]),
                                )
                            }
                            tools::CanonicalTool::Grep { pattern, path } => {
                                let qpat = tools::shell_quote(&pattern);
                                let cmd = if let Some(p) = path.as_deref().filter(|s| !s.is_empty())
                                {
                                    format!("rg -n {qpat} {}", tools::shell_quote(p))
                                } else {
                                    format!("rg -n {qpat}")
                                };
                                (
                                    cmd.clone(),
                                    json!([{"type":"unknown","cmd":cmd}]),
                                    json!([{"type":"unknown","command":cmd}]),
                                )
                            }
                            _ => unreachable!(),
                        };
                        let proc_id =
                            format!("{}", 10000 + (rollout.ordinal as u64 * 7919) % 80000);
                        let shell_argv = if cfg!(windows) {
                            json!(["powershell.exe", "-Command", cmd_str])
                        } else {
                            json!(["/bin/sh", "-lc", cmd_str])
                        };
                        let fn_args = serde_json::to_string(&json!({"cmd":cmd_str,"workdir":cwd}))?;
                        rollout.emit(
                            &call_ts,
                            "response_item",
                            json!({"type":"function_call","status":"completed","id":fc_id,"name":"exec_command","arguments":fn_args,"call_id":call_id}),
                        )?;
                        let (ord, _, _) = rollout.emit(
                            &res_ts,
                            "event_msg",
                            json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"CommandExecution","id":item_id,"process_id":proc_id,"command":shell_argv,"cwd":cwd,"parsed_cmd":parsed_cmd,"source":"unified_exec_startup","status":status_str,"aggregated_output":inv.result_text,"exit_code":if inv.is_error{1}else{0},"duration":{"secs":dur_ms/1000,"nanos":(dur_ms%1000)*1_000_000}},"started_at_ms":c_ms,"completed_at_ms":r_ms}),
                        )?;
                        rollout.emit(
                            &res_ts,
                            "response_item",
                            json!({"type":"function_call_output","id":fco_id,"call_id":call_id,"output":inv.result_text}),
                        )?;
                        proj_items.push(ProjItem {
                            turn: turn.clone(),
                            id: item_id.clone(),
                            ordinal: ord,
                            ms: c_ms,
                            item: json!({"type":"commandExecution","id":item_id,"pluginId":null,"scriptPath":null,"command":cmd_str,"cwd":cwd,"processId":proc_id,"source":"unifiedExecStartup","status":status_str,"commandActions":command_actions,"aggregatedOutput":inv.result_text,"exitCode":if inv.is_error{1}else{0},"durationMs":dur_ms}),
                            kind: "commandExecution".into(),
                        });
                    }
                    tools::CanonicalTool::WebSearch { query } => {
                        let fn_args = serde_json::to_string(&json!({"query":query}))?;
                        rollout.emit(
                            &call_ts,
                            "response_item",
                            json!({"type":"function_call","status":"completed","id":fc_id,"name":"web_search","arguments":fn_args,"call_id":call_id}),
                        )?;
                        let (ord, _, _) = rollout.emit(
                            &res_ts,
                            "event_msg",
                            json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"WebSearch","id":item_id,"query":query,"action":{"type":"search","query":query,"queries":null}},"started_at_ms":c_ms,"completed_at_ms":r_ms}),
                        )?;
                        rollout.emit(
                            &res_ts,
                            "response_item",
                            json!({"type":"function_call_output","id":fco_id,"call_id":call_id,"output":inv.result_text}),
                        )?;
                        proj_items.push(ProjItem {
                            turn: turn.clone(),
                            id: item_id.clone(),
                            ordinal: ord,
                            ms: c_ms,
                            item: json!({"type":"webSearch","id":item_id,"query":query,"action":{"type":"search","query":query,"queries":null},"results":null}),
                            kind: "webSearch".into(),
                        });
                    }
                    tools::CanonicalTool::Generic { name, args } => {
                        let args_obj = if args.is_object() { args } else { json!({}) };
                        let (server, tool_short) = if let Some(rest) = name.strip_prefix("mcp__") {
                            if let Some((srv, tl)) = rest.split_once("__") {
                                (srv.to_owned(), tl.to_owned())
                            } else {
                                ("tools".to_owned(), name.clone())
                            }
                        } else {
                            ("tools".to_owned(), name.clone())
                        };
                        let fn_args = serde_json::to_string(&args_obj)?;
                        rollout.emit(
                            &call_ts,
                            "response_item",
                            json!({"type":"function_call","status":"completed","id":fc_id,"name":name,"arguments":fn_args,"call_id":call_id}),
                        )?;
                        let ev_result = if inv.is_error {
                            Value::Null
                        } else {
                            json!({"content":[{"type":"text","text":inv.result_text}],"isError":false})
                        };
                        let (ord, _, _) = rollout.emit(
                            &res_ts,
                            "event_msg",
                            json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"McpToolCall","id":item_id,"server":server,"tool":tool_short,"arguments":args_obj,"status":status_str,"result":ev_result,"duration":{"secs":dur_ms/1000,"nanos":(dur_ms%1000)*1_000_000}},"started_at_ms":c_ms,"completed_at_ms":r_ms}),
                        )?;
                        rollout.emit(
                            &res_ts,
                            "response_item",
                            json!({"type":"function_call_output","id":fco_id,"call_id":call_id,"output":inv.result_text}),
                        )?;
                        let proj_result = if inv.is_error {
                            Value::Null
                        } else {
                            json!({"content":[{"type":"text","text":inv.result_text}],"structuredContent":null,"_meta":null})
                        };
                        let proj_err = if inv.is_error {
                            json!({"message":inv.result_text})
                        } else {
                            Value::Null
                        };
                        proj_items.push(ProjItem {
                            turn: turn.clone(),
                            id: item_id.clone(),
                            ordinal: ord,
                            ms: c_ms,
                            item: json!({"type":"mcpToolCall","id":item_id,"server":server,"environmentId":null,"tool":tool_short,"status":status_str,"arguments":args_obj,"appContext":null,"mcpAppUi":null,"pluginId":null,"readOnlyHint":null,"result":proj_result,"error":proj_err,"durationMs":dur_ms}),
                            kind: "mcpToolCall".into(),
                        });
                    }
                }
            } else if kind == "compact" {
                pos += 1;
                let id = format!("cmp_{}", &uuid_hex()[..16]);
                let (ord,_,_)=rollout.emit(&ts,"event_msg",json!({"type":"item_completed","thread_id":tid,"turn_id":turn,"item":{"type":"ContextCompaction","id":id},"started_at_ms":ms,"completed_at_ms":ms}))?;
                rollout.emit(&ts,"response_item",json!({"type":"message","id":format!("msg_{}",uuid7(ms)),"role":"developer","content":[{"type":"input_text","text":text}]}))?;
                proj_items.push(ProjItem {
                    turn: turn.clone(),
                    id: id.clone(),
                    ordinal: ord,
                    ms,
                    item: json!({"type":"contextCompaction","id":id}),
                    kind: "contextCompaction".into(),
                });
            } else if kind == "error" {
                pos += 1;
                turn_error = Some(json!({"message":text,"codex_error_info":"other"}));
            } else {
                pos += 1;
                rollout.emit(&ts,"response_item",json!({"type":"message","id":format!("msg_{}",uuid7(ms)),"role":"developer","content":[{"type":"input_text","text":text}]}))?;
            }
        }
        let mut complete = json!({"type":"task_complete","root_turn_id":turn,"turn_id":turn,"last_agent_message":agent_text,
            "started_at":start_sec as i64,"completed_at":end_sec as i64,"duration_ms":(end_ms-start_ms).max(1)});
        if let Some(e) = &turn_error {
            complete["error"] = e.clone();
        }
        let (end_ord, _, end_off) = rollout.emit(&iso(end_sec), "event_msg", complete)?;
        proj_turns.push(ProjTurn {
            turn,
            start: start_ord,
            status: if turn_error.is_some() {
                "failed"
            } else {
                "completed"
            }
            .into(),
            error: turn_error.map(|e| e.to_string()),
            started: start_sec as i64,
            completed: end_sec as i64,
            duration: (end_ms - start_ms).max(1),
            user: user_id,
            agent: agent_id,
            offset: start_off,
            end: end_ord,
            end_offset: end_off,
        });
    }
    atomic_write(&path, &rollout.bytes)?;
    let session_index = home.join("session_index.jsonl");
    let before_index = fs::read(&session_index).ok();
    let mut index = before_index.clone().unwrap_or_default();
    if !index.is_empty() && !index.ends_with(b"\n") {
        index.push(b'\n');
    }
    serde_json::to_writer(
        &mut index,
        &json!({"id":tid,"thread_name":title,"updated_at":iso(last.max(now))}),
    )?;
    index.push(b'\n');
    atomic_write(&session_index, &index)?;
    guard.index(session_index, "id", &tid, before_index, index);
    let state = home.join("state_5.sqlite");
    if state.is_file() {
        let mut c = connection(&state)?;
        let cols = {
            let mut st = c.prepare("PRAGMA table_info(threads)")?;
            let rows = st
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, bool>(3)?,
                        r.get::<_, Option<String>>(4)?,
                    ))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            rows
        };
        let fullpath = path.canonicalize()?.to_string_lossy().into_owned();
        let mut cwd_db = cwd.clone();
        if cfg!(windows) && !cwd_db.starts_with(r"\\") {
            cwd_db = windows_cwd(&Path::new(&cwd).canonicalize()?.to_string_lossy());
        }
        let created = first as i64;
        let updated = last.max(now) as i64;
        let first_message = if first_user.is_empty() {
            &title
        } else {
            &first_user
        };
        let defaults = json!({"id":tid,"rollout_path":fullpath,"created_at":created,"updated_at":updated,"source":tpl["source"],
            "model_provider":tpl["model_provider"],"cwd":cwd_db,"title":title,"sandbox_policy":tpl["sandbox_policy"],"approval_mode":tpl["approval_mode"],
            "tokens_used":0,"has_user_event":0,"archived":0,"cli_version":tpl["cli_version"],"first_user_message":first_message,"memory_mode":"enabled",
            "model":model,"reasoning_effort":effort,"created_at_ms":created*1000,"updated_at_ms":updated*1000,"thread_source":"user",
            "preview":shorten(first_message,200),"recency_at":updated,"recency_at_ms":updated*1000,"history_mode":"paginated","name":title,"is_pinned":0,
            "originator":tpl["originator"],"creator_user_id":tpl["creator_user_id"],"creator_account_id":tpl["creator_account_id"]});
        let mut names = Vec::new();
        let mut vals = Vec::new();
        for (name, ty, notnull, default) in cols {
            if let Some(value) = defaults.get(&name) {
                names.push(name);
                vals.push(json_sql(value));
            } else if notnull && default.is_none() {
                names.push(name);
                vals.push(if ty.to_uppercase().contains("INT") {
                    SqlValue::Integer(0)
                } else {
                    SqlValue::Text(String::new())
                });
            }
        }
        if !names.is_empty() {
            let quoted = names
                .iter()
                .map(|n| format!("\"{}\"", n.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(",");
            let placeholders = vec!["?"; names.len()].join(",");
            let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            tx.execute(
                &format!("INSERT OR REPLACE INTO threads({quoted}) VALUES ({placeholders})"),
                params_from_iter(vals),
            )?;
            tx.commit()?;
            guard.row(state.clone(), "threads", "id", &tid);
        }
    }
    let proj = home.join("thread_history_1.sqlite");
    if proj.is_file() || !default_home {
        let mut c = connection(&proj)?;
        projection_schema(&c)?;
        let tx = c.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        tx.execute("DELETE FROM thread_items WHERE thread_id=?", [&tid])?;
        tx.execute("DELETE FROM thread_turns WHERE thread_id=?", [&tid])?;
        for t in proj_turns {
            tx.execute("INSERT OR REPLACE INTO thread_turns(thread_id,turn_id,rollout_ordinal,status,error_json,started_at,completed_at,duration_ms,first_user_item_id,final_agent_item_id,rollout_byte_offset,rollout_end_ordinal,rollout_end_byte_offset,root_turn_id) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
            params![tid,t.turn,t.start,t.status,t.error,t.started,t.completed,t.duration,t.user,t.agent,t.offset,t.end,t.end_offset,t.turn])?;
        }
        for i in proj_items {
            tx.execute("INSERT OR REPLACE INTO thread_items(thread_id,turn_id,item_id,rollout_ordinal,created_at_ms,item_json,item_type,updated_at_ordinal,started_at_ms,completed_at_ms) VALUES (?,?,?,?,?,?,?,?,?,?)",
            params![tid,i.turn,i.id,i.ordinal,i.ms,serde_json::to_string(&i.item)?,i.kind,i.ordinal,i.ms,i.ms])?;
        }
        tx.execute("INSERT OR REPLACE INTO thread_history_projection_state(thread_id,next_rollout_byte_offset,next_rollout_ordinal) VALUES (?,?,?)",params![tid,rollout.bytes.len() as i64,rollout.ordinal])?;
        tx.commit()?;
    }
    let _ = update_global(home, &tid, &cwd, &roots);
    guard.commit();
    Ok(json!({"provider":"codex","cid":tid,"title":title,"entries":entries.len()}))
}

#[cfg(test)]
mod path_tests {
    use super::*;
    #[test]
    fn global_project_matching_normalizes_missing_paths_lexically() {
        let home = tempfile::tempdir().unwrap();
        let cwd = if cfg!(windows) {
            r"C:\k1tty-uninstalled\project"
        } else {
            "/k1tty-uninstalled/project"
        };
        let path = home.path().join(".codex-global-state.json");
        write_json(
            &path,
            &json!({"local-projects":{"existing":{"rootPaths":[format!("{cwd}/missing/../")]}}}),
        )
        .unwrap();
        update_global(home.path(), "new-thread", cwd, &[]).unwrap();
        let state: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(
            state["thread-project-assignments"]["new-thread"],
            json!({"projectKind":"local","projectId":"existing"})
        );
        assert!(state["projectless-thread-ids"].is_null());
    }

    #[test]
    fn windows_cwd_has_one_prefix_and_preserves_unc() {
        assert_eq!(windows_cwd(r"C:\Users\тест"), r"\\?\C:\Users\тест");
        assert_eq!(windows_cwd(r"\\?\C:\Users\тест"), r"\\?\C:\Users\тест");
        assert_eq!(
            windows_cwd(r"\\server\share\проект"),
            r"\\server\share\проект"
        );
        assert_eq!(
            windows_cwd(r"\\?\UNC\server\share\проект"),
            r"\\server\share\проект"
        );
    }
}
