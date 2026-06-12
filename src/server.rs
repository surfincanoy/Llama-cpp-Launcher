use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::i18n::Lang;

pub struct ServerProcess {
    child: Child,
}

impl ServerProcess {
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub enum ServerEvent {
    Log(String),
    Started(ServerProcess),
    Failed(String),
}


#[allow(clippy::too_many_arguments)]
pub fn start_server_async(
    executable: String,
    model_dir: String,
    model_name: String,
    host: String,
    port: u16,
    n_gpu_layers: u32,
    ctx_size: u32,
    mtp_enabled: bool,
    flash_attn: String,
    spec_draft_n_max: u32,
    mmproj: String,
    extra_args: String,
    lang: Lang,
    log_sender: mpsc::Sender<ServerEvent>,
) {
    std::thread::spawn(move || {
        let send = |msg: &str| {
            let _ = log_sender.send(ServerEvent::Log(msg.to_string()));
        };

        let model_path = match resolve_model_path(&model_dir, &model_name, lang) {
            Ok(p) => p,
            Err(e) => {
                send(&e);
                let _ = log_sender.send(ServerEvent::Failed(e));
                return;
            }
        };

        if !std::path::Path::new(&executable).exists() {
            let e = match lang {
                Lang::Zh => format!("可执行文件不存在: {}", executable),
                Lang::En => format!("Executable not found: {}", executable),
            };
            send(&e);
            let _ = log_sender.send(ServerEvent::Failed(e));
            return;
        }

        let mut cmd = Command::new(&executable);
        cmd.arg("-m")
            .arg(&model_path)
            .arg("--host")
            .arg(&host)
            .arg("--port")
            .arg(port.to_string())
            .arg("-c")
            .arg(ctx_size.to_string())
            .arg("--n-gpu-layers")
            .arg(n_gpu_layers.to_string());
        if mtp_enabled {
            cmd.arg("--spec-type").arg("draft-mtp")
                .arg("--spec-draft-n-max").arg(spec_draft_n_max.to_string());
        }
        if flash_attn != "auto" {
            cmd.arg(format!("--flash-attn={}", flash_attn));
        }
        if !mmproj.is_empty() {
            let mmproj_path = resolve_model_path(&model_dir, &mmproj, lang)
                .unwrap_or_else(|_| mmproj.clone());
            cmd.arg("--mmproj").arg(&mmproj_path);
        }
        if !extra_args.is_empty() {
            for arg in extra_args.split_whitespace() {
                cmd.arg(arg);
            }
        }
        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        kill_process_on_port(port);

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                let msg = match lang {
                    Lang::Zh => format!("启动失败: {}", e),
                    Lang::En => format!("Start failed: {}", e),
                };
                send(&msg);
                let _ = log_sender.send(ServerEvent::Failed(msg));
                return;
            }
        };

        let pid = child.id();
        let started_msg = match lang {
            Lang::Zh => format!("llama-server 已启动, PID: {}", pid),
            Lang::En => format!("llama-server started, PID: {}", pid),
        };
        send(&started_msg);

        let (log_tx, log_rx) = mpsc::channel::<String>();

        if let Some(out) = child.stdout.take() {
            let tx = log_tx.clone();
            std::thread::spawn(move || {
                let reader = BufReader::new(out);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = tx.send(line);
                }
            });
        }
        if let Some(err) = child.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(err);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = log_tx.send(line);
                }
            });
        }

        let url = format!("http://{}:{}/v1/models", host, port);
        let start = Instant::now();
        let timeout = Duration::from_secs(120);

        let config = ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(2)))
            .build();
        let agent = ureq::Agent::new_with_config(config);

        let mut ready = false;
        let mut timed_out = false;

        loop {
            while let Ok(line) = log_rx.try_recv() {
                send(&line);
            }

            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) => {}
                Err(_) => break,
            }

            if start.elapsed() >= timeout {
                timed_out = true;
                break;
            }

            if agent.get(&url).call().map(|r| r.status() == 200).unwrap_or(false) {
                // Let any pending exit complete before trusting the health check
                std::thread::sleep(Duration::from_millis(200));
                match child.try_wait() {
                    Ok(None) => {
                        ready = true;
                        // Collect remaining startup logs
                        for _ in 0..5 {
                            std::thread::sleep(Duration::from_millis(50));
                            while let Ok(line) = log_rx.try_recv() {
                                send(&line);
                            }
                        }
                        break;
                    }
                    _ => break,
                }
            }

            std::thread::sleep(Duration::from_millis(200));
        }

        // Drain any remaining log lines
        for line in log_rx.try_iter() {
            send(&line);
        }

        if ready {
            let ready_msg = match lang {
                Lang::Zh => format!("llama-server 已就绪: http://{}:{}", host, port),
                Lang::En => format!("llama-server ready: http://{}:{}", host, port),
            };
            send(&ready_msg);
            let _ = log_sender.send(ServerEvent::Started(ServerProcess { child }));
        } else {
            let _ = child.kill();
            let _ = child.wait();
            let fail_msg = if timed_out {
                match lang {
                    Lang::Zh => format!("启动失败: 等待 {}:{} 超时 ({}s)", host, port, timeout.as_secs()),
                    Lang::En => format!("start failed: waiting for {}:{} timeout ({}s)", host, port, timeout.as_secs()),
                }
            } else {
                match lang {
                    Lang::Zh => format!("启动失败: 端口 {}:{} 被占用", host, port),
                    Lang::En => format!("start failed: port {}:{} in use", host, port),
                }
            };
            let _ = log_sender.send(ServerEvent::Failed(fail_msg));
        }
    });
}

fn kill_process_on_port(port: u16) {
    let output = std::process::Command::new("fuser")
        .arg("-n")
        .arg("tcp")
        .arg(port.to_string())
        .stderr(std::process::Stdio::null())
        .output();
    let pid_str = match &output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => return,
    };
    if pid_str.is_empty() {
        return;
    }
    for pid in pid_str.split_whitespace() {
        if let Ok(pid) = pid.parse::<u32>() {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .status();
        }
    }
    std::thread::sleep(Duration::from_millis(200));
}

pub(crate) fn resolve_model_path(model_dir: &str, model_name: &str, lang: Lang) -> Result<String, String> {
    let dir = std::path::Path::new(model_dir);
    let direct = dir.join(model_name);
    if direct.exists() {
        return Ok(direct.to_string_lossy().to_string());
    }
    if !model_name.ends_with(".gguf") {
        let with_ext = dir.join(format!("{}.gguf", model_name));
        if with_ext.exists() {
            return Ok(with_ext.to_string_lossy().to_string());
        }
    }
    Err(match lang {
        Lang::Zh => format!("模型文件不存在: {}/{}", model_dir, model_name),
        Lang::En => format!("Model file not found: {}/{}", model_dir, model_name),
    })
}

pub fn stop_server(process: &mut ServerProcess, lang: Lang) -> Vec<String> {
    let mut logs = Vec::new();
    match process.child.terminate() {
        Ok(_) => match process.child.wait_timeout(Duration::from_secs(5)) {
            Ok(Some(status)) => {
                logs.push(match lang {
                    Lang::Zh => format!("llama-server 已停止 (exit: {})", status),
                    Lang::En => format!("llama-server stopped (exit: {})", status),
                });
            }
            Ok(None) => {
                logs.push(match lang {
                    Lang::Zh => "llama-server 未响应, 强制终止".to_string(),
                    Lang::En => "llama-server no response, killing".to_string(),
                });
                process.kill();
                logs.push(match lang {
                    Lang::Zh => "llama-server 已强制停止".to_string(),
                    Lang::En => "llama-server force stopped".to_string(),
                });
            }
            Err(e) => {
                logs.push(match lang {
                    Lang::Zh => format!("等待退出失败: {}, 强制终止", e),
                    Lang::En => format!("Wait exit failed: {}, killing", e),
                });
                process.kill();
                logs.push(match lang {
                    Lang::Zh => "llama-server 已强制停止".to_string(),
                    Lang::En => "llama-server force stopped".to_string(),
                });
            }
        },
        Err(e) => {
            logs.push(match lang {
                Lang::Zh => format!("停止失败: {}, 强制终止", e),
                Lang::En => format!("Stop failed: {}, killing", e),
            });
            process.kill();
            logs.push(match lang {
                Lang::Zh => "llama-server 已强制停止".to_string(),
                Lang::En => "llama-server force stopped".to_string(),
            });
        }
    }
    logs
}

trait CommandExt {
    fn terminate(&mut self) -> std::io::Result<()>;
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl CommandExt for Child {
    fn terminate(&mut self) -> std::io::Result<()> {
        #[cfg(unix)]
        unsafe {
            libc::kill(self.id() as i32, libc::SIGTERM);
        }
        #[cfg(windows)]
        {
            let _ = self.kill();
        }
        Ok(())
    }

    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = Instant::now();
        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }
}
