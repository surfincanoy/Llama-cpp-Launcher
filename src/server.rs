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
            .arg(n_gpu_layers.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

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

        let host_clone = host.clone();
        let wait_thread = std::thread::spawn(move || wait_for_server(&host_clone, port, 30));

        while let Ok(line) = log_rx.recv() {
            send(&line);
        }

        let ready = wait_thread.join().unwrap_or(false);

        if ready {
            let ready_msg = match lang {
                Lang::Zh => format!("llama-server 已就绪, 监听 {}:{}", host, port),
                Lang::En => format!("llama-server ready, listening on {}:{}", host, port),
            };
            send(&ready_msg);
            let _ = log_sender.send(ServerEvent::Started(ServerProcess { child }));
        } else {
            let timeout_msg = match lang {
                Lang::Zh => "llama-server 启动超时",
                Lang::En => "llama-server start timeout",
            };
            send(timeout_msg);
            let _ = child.kill();
            let _ = child.wait();
            let fail_msg = match lang {
                Lang::Zh => "启动超时",
                Lang::En => "start timeout",
            };
            let _ = log_sender.send(ServerEvent::Failed(fail_msg.to_string()));
        }
    });
}

fn resolve_model_path(model_dir: &str, model_name: &str, lang: Lang) -> Result<String, String> {
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

fn wait_for_server(host: &str, port: u16, timeout_secs: u64) -> bool {
    let url = format!("http://{}:{}/v1/models", host, port);
    let start = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    let config = ureq::config::Config::builder()
        .timeout_global(Some(Duration::from_secs(2)))
        .build();
    let agent = ureq::Agent::new_with_config(config);

    while start.elapsed() < timeout {
        if let Ok(resp) = agent.get(&url).call() {
            if resp.status() == 200 {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    false
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
