# Interception 驱动文件目录

此目录用于存放 Interception 驱动安装文件（DESIGN 12.3 / REQUIREMENTS 3.11）。

## 所需文件

- `install-interception.exe` — 官方安装器
- `interception.dll` — 驱动库文件（阶段 13 运行时加载）

## 安装说明

应用会通过 `ShellExecuteW("runas")` 以管理员身份调用：

```
install-interception.exe /install
```

安装完成后**必须重启系统**，驱动才会加载。

## 状态

> **待确认事项 #4**：驱动文件与安装命令待用户放入后最终确认。
