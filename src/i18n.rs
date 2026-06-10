#[derive(Clone, Copy, PartialEq)]
pub enum Lang {
    Zh,
    En,
}

fn detect_lang() -> Lang {
    if let Some(locale) = sys_locale::get_locale() {
        if locale.starts_with("zh") {
            return Lang::Zh;
        }
    }
    Lang::En
}

macro_rules! t_str {
    ($name:ident, $zh:expr, $en:expr) => {
        pub fn $name(&self) -> &str {
            match self.lang {
                Lang::Zh => $zh,
                Lang::En => $en,
            }
        }
    };
}

pub struct L10n {
    lang: Lang,
}

impl L10n {
    pub fn detect() -> Self {
        Self {
            lang: detect_lang(),
        }
    }

    pub fn lang(&self) -> Lang {
        self.lang
    }
}

impl L10n {
    t_str!(config_title, "⚙  服务配置", "⚙  Server Config");
    t_str!(executable, "可执行文件", "Executable");
    t_str!(browse, "浏览", "Browse");
    t_str!(model_dir, "模型目录", "Model Directory");
    t_str!(model_file, "模型文件", "Model File");
    t_str!(host, "监听地址", "Host");
    t_str!(port, "端口", "Port");
    t_str!(gpu_layers, "GPU 层数", "GPU Layers");
    t_str!(ctx_size, "上下文大小", "Context Size");
    t_str!(save_config, "💾  保存配置", "💾  Save Config");
    t_str!(start_server, "▶  启动服务", "▶  Start");
    t_str!(stop_server, "⏹  停止服务", "⏹  Stop");
    t_str!(logs, "日志:", "Logs:");
    t_str!(not_running, "未运行", "Not Running");
    t_str!(starting, "启动中...", "Starting...");
    t_str!(start_failed, "启动失败", "Start Failed");
    t_str!(starting_server, "正在启动 llama-server...", "Starting llama-server...");
    t_str!(stopping_server, "正在停止 llama-server...", "Stopping llama-server...");
    t_str!(exec_hint, "选择 llama-server ...", "Select llama-server ...");
    t_str!(dir_hint, "选择模型文件夹 ...", "Select model folder ...");

    pub fn running_with_pid(&self, pid: u32) -> String {
        match self.lang {
            Lang::Zh => format!("运行中 (PID: {})", pid),
            Lang::En => format!("Running (PID: {})", pid),
        }
    }

    pub fn failed(&self, msg: &str) -> String {
        match self.lang {
            Lang::Zh => format!("失败: {}", msg),
            Lang::En => format!("Failed: {}", msg),
        }
    }

    t_str!(exec_filter, "可执行文件", "Executable");


}
