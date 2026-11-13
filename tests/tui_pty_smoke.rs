use std::io::Write;
use std::net::TcpListener;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn tui_runs_in_pty_and_exits_on_q() {
    // best-effort: requires python3 and a usable PTY (typical on CI/Linux dev)
    if Command::new("python3").arg("--version").output().is_err() {
        eprintln!("skipping: python3 not available");
        return;
    }

    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("t.db");
    let cfg_path = dir.path().join("config.toml");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind port");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let mut f = std::fs::File::create(&cfg_path).expect("config");
    write!(
        f,
        r#"bind="127.0.0.1"
port={port}
db_path="{}"
interval_seconds=60
retention_hours=1
request_timeout_secs=1
max_history_hours=1

[[targets]]
name = "local"
url = "http://127.0.0.1:9/"
kind = "generic"
"#,
        db.display()
    )
    .expect("write config");

    let bin = env!("CARGO_BIN_EXE_tinytop");
    let script = r#"
import os, pty, select, time, sys, subprocess
bin, cfg = sys.argv[1], sys.argv[2]
master, slave = pty.openpty()
env = os.environ.copy()
env['TERM'] = 'xterm-256color'
p = subprocess.Popen([bin,'--tui','--config',cfg], stdin=slave, stdout=slave, stderr=slave, close_fds=True, env=env)
os.close(slave)
out=b''
start=time.time(); sent=False
try:
    while time.time()-start < 12:
        r,_,_ = select.select([master],[],[],0.4)
        if r:
            try:
                ch=os.read(master,65536)
            except OSError:
                break
            if not ch: break
            out += ch
            if (not sent) and len(out) > 100:
                time.sleep(0.8); os.write(master,b'q'); sent=True
        if p.poll() is not None:
            while True:
                r,_,_ = select.select([master],[],[],0.1)
                if not r: break
                try:
                    ch=os.read(master,65536)
                except OSError:
                    ch=b''
                if not ch: break
                out+=ch
            break
    else:
        try: os.write(master,b'q')
        except: pass
finally:
    try: os.close(master)
    except: pass
try:
    p.wait(timeout=5)
except Exception:
    p.kill(); p.wait()
sys.exit(0 if p.returncode==0 and len(out)>50 else 1)
"#;

    let status = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(bin)
        .arg(&cfg_path)
        .status()
        .expect("run pty smoke");
    assert!(status.success(), "tui PTY smoke failed: {status}");
}
