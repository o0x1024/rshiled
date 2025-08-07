# 代码签名配置指南

本文档说明如何为 rshield 应用配置代码签名，以确保应用的安全性和可信度。

## 概述

代码签名是一个重要的安全措施，它可以：
- 验证应用的完整性
- 确认应用来源的可信度
- 防止应用被恶意修改
- 在某些平台上避免安全警告

## 平台支持

当前配置支持以下平台的代码签名：
- **macOS**: 使用 Apple Developer 证书
- **Windows**: 使用代码签名证书
- **Linux**: 目前不需要特殊签名配置

## GitHub Secrets 配置

### 通用配置

在 GitHub 仓库的 Settings > Secrets and variables > Actions 中添加以下 secrets：

#### Tauri 更新器签名
```
TAURI_PRIVATE_KEY=<your-tauri-private-key>
TAURI_KEY_PASSWORD=<your-tauri-key-password>
```

### macOS 代码签名

#### 必需的 Secrets
```
APPLE_CERTIFICATE=<base64-encoded-p12-certificate>
APPLE_CERTIFICATE_PASSWORD=<certificate-password>
APPLE_SIGNING_IDENTITY=<signing-identity-name>
APPLE_TEAM_ID=<your-apple-team-id>
```

#### 公证相关 (可选，用于分发)
```
APPLE_ID=<your-apple-id-email>
APPLE_PASSWORD=<app-specific-password>
```

#### 获取 macOS 证书
1. 在 Apple Developer 账户中创建 "Developer ID Application" 证书
2. 下载证书并导出为 .p12 格式
3. 将 .p12 文件转换为 base64：
   ```bash
   base64 -i certificate.p12 -o certificate.txt
   ```
4. 将 certificate.txt 的内容作为 `APPLE_CERTIFICATE` 的值

### Windows 代码签名

#### 必需的 Secrets
```
WINDOWS_CERTIFICATE=<base64-encoded-p12-certificate>
WINDOWS_CERTIFICATE_PASSWORD=<certificate-password>
```

#### 获取 Windows 证书
1. 从证书颁发机构（如 DigiCert、Sectigo 等）购买代码签名证书
2. 将证书导出为 .p12 格式
3. 将 .p12 文件转换为 base64：
   ```bash
   base64 -i certificate.p12 -o certificate.txt
   ```
4. 将 certificate.txt 的内容作为 `WINDOWS_CERTIFICATE` 的值

## 生成 Tauri 更新器密钥

如果还没有 Tauri 更新器密钥，可以使用以下命令生成：

```bash
# 安装 Tauri CLI
cargo install tauri-cli

# 生成密钥对
tauri signer generate -w ~/.tauri/myapp.key
```

这将生成：
- 私钥文件：`~/.tauri/myapp.key`
- 公钥：显示在终端输出中

将私钥文件的内容作为 `TAURI_PRIVATE_KEY` 的值。

## 配置验证

### 本地测试

在配置完成后，可以在本地测试构建：

```bash
# 构建应用（不签名）
yarn tauri build

# 如果配置了签名环境变量，构建时会自动签名
export TAURI_PRIVATE_KEY="your-private-key"
export TAURI_KEY_PASSWORD="your-password"
yarn tauri build
```

### CI/CD 验证

推送代码到 GitHub 后，检查 Actions 页面的构建日志：

1. 确认签名步骤执行成功
2. 检查构建产物是否正确签名
3. 验证发布的应用可以正常安装和运行

## 故障排除

### 常见问题

1. **macOS 签名失败**
   - 检查证书是否为 "Developer ID Application" 类型
   - 确认证书未过期
   - 验证 Team ID 是否正确

2. **Windows 签名失败**
   - 确认证书格式正确（.p12）
   - 检查证书密码是否正确
   - 验证证书是否为代码签名证书

3. **Tauri 更新器签名失败**
   - 检查私钥格式是否正确
   - 确认密钥密码是否正确

### 调试技巧

1. 在 GitHub Actions 中启用调试模式：
   ```yaml
   env:
     ACTIONS_STEP_DEBUG: true
   ```

2. 检查证书信息：
   ```bash
   # macOS
   security find-identity -v -p codesigning
   
   # Windows
   certlm.msc
   ```

## 安全注意事项

1. **保护私钥和证书**
   - 永远不要将私钥或证书提交到代码仓库
   - 使用 GitHub Secrets 安全存储敏感信息
   - 定期轮换证书和密钥

2. **访问控制**
   - 限制对 GitHub Secrets 的访问权限
   - 使用最小权限原则
   - 定期审查访问日志

3. **证书管理**
   - 监控证书过期时间
   - 提前续费证书
   - 备份证书和私钥

## 更新和维护

1. **定期检查**
   - 每月检查证书状态
   - 验证签名流程是否正常
   - 更新过期的证书

2. **文档更新**
   - 记录证书更新过程
   - 更新相关配置文档
   - 通知团队成员

## 参考资源

- [Tauri 代码签名文档](https://tauri.app/v1/guides/distribution/sign-your-application)
- [Apple 代码签名指南](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Windows 代码签名最佳实践](https://docs.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools)