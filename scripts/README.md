# 脚本工具

本目录包含用于 rshield 项目的各种实用脚本。

## 可用脚本

### setup-signing.sh
**用途**: 帮助开发者快速设置本地代码签名环境

**功能**:
- 自动检测操作系统
- 生成 Tauri 更新器密钥对
- 提供平台特定的代码签名设置指导
- 生成环境变量配置模板

**使用方法**:
```bash
./scripts/setup-signing.sh
```

### verify-signing.sh
**用途**: 验证代码签名配置是否正确

**功能**:
- 检查必需的工具依赖（Tauri CLI、Cargo、Node.js）
- 验证环境变量配置
- 检查平台特定的签名配置
- 验证项目配置文件
- 提供详细的配置状态报告

**使用方法**:
```bash
./scripts/verify-signing.sh
```

## 使用流程

1. **首次设置**:
   ```bash
   # 运行设置脚本
   ./scripts/setup-signing.sh
   
   # 根据输出配置环境变量
   # 编辑 ~/.zshrc 或 ~/.bashrc
   ```

2. **验证配置**:
   ```bash
   # 验证所有配置是否正确
   ./scripts/verify-signing.sh
   ```

3. **测试构建**:
   ```bash
   # 本地构建测试
   npm run tauri build
   ```

## 环境变量说明

### 通用变量
- `TAURI_PRIVATE_KEY`: Tauri 更新器私钥
- `TAURI_KEY_PASSWORD`: 私钥密码

### macOS 变量
- `APPLE_CERTIFICATE`: Base64 编码的 .p12 证书
- `APPLE_CERTIFICATE_PASSWORD`: 证书密码
- `APPLE_SIGNING_IDENTITY`: 签名身份
- `APPLE_TEAM_ID`: Apple 团队 ID
- `APPLE_ID`: Apple ID（用于公证）
- `APPLE_PASSWORD`: App 专用密码（用于公证）

### Windows 变量
- `WINDOWS_CERTIFICATE`: Base64 编码的 .p12 证书
- `WINDOWS_CERTIFICATE_PASSWORD`: 证书密码

## 故障排除

### 常见问题

1. **权限错误**:
   ```bash
   chmod +x scripts/*.sh
   ```

2. **找不到 Tauri CLI**:
   ```bash
   cargo install tauri-cli
   ```

3. **环境变量未生效**:
   ```bash
   source ~/.zshrc  # 或 ~/.bashrc
   ```

### 获取帮助

- 查看详细配置指南: [../docs/code-signing.md](../docs/code-signing.md)
- 检查 GitHub Actions 构建日志
- 运行验证脚本获取详细状态信息

## 注意事项

⚠️ **安全提醒**:
- 永远不要将私钥或证书提交到代码仓库
- 使用环境变量或 GitHub Secrets 存储敏感信息
- 定期更新和轮换证书
- 限制对签名材料的访问权限