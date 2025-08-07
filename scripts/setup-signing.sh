#!/bin/bash

# 代码签名设置脚本
# 用于帮助开发者快速配置本地代码签名环境

set -e

echo "🔐 rshield 代码签名设置脚本"
echo "================================"

# 检查操作系统
OS="$(uname -s)"
case "${OS}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    CYGWIN*)    MACHINE=Cygwin;;
    MINGW*)     MACHINE=MinGw;;
    *)          MACHINE="UNKNOWN:${OS}"
esac

echo "检测到操作系统: ${MACHINE}"

# 创建配置目录
CONFIG_DIR="$HOME/.tauri"
mkdir -p "$CONFIG_DIR"

# 生成 Tauri 更新器密钥
echo ""
echo "📝 生成 Tauri 更新器密钥..."
if command -v tauri &> /dev/null; then
    if [ ! -f "$CONFIG_DIR/rshield.key" ]; then
        echo "生成新的 Tauri 密钥对..."
        tauri signer generate -w "$CONFIG_DIR/rshield.key"
        echo "✅ 密钥已生成: $CONFIG_DIR/rshield.key"
        echo "⚠️  请将公钥添加到 tauri.conf.json 的 updater 配置中"
    else
        echo "✅ Tauri 密钥已存在: $CONFIG_DIR/rshield.key"
    fi
else
    echo "❌ 未找到 tauri CLI，请先安装:"
    echo "   cargo install tauri-cli"
    exit 1
fi

# 平台特定设置
case "${MACHINE}" in
    Mac)
        echo ""
        echo "🍎 macOS 代码签名设置"
        echo "====================="
        echo "1. 确保你有 Apple Developer 账户"
        echo "2. 在 Keychain Access 中安装 'Developer ID Application' 证书"
        echo "3. 检查可用的签名身份:"
        echo "   security find-identity -v -p codesigning"
        echo ""
        echo "4. 导出证书为 .p12 格式:"
        echo "   - 在 Keychain Access 中右键点击证书"
        echo "   - 选择 'Export'"
        echo "   - 选择 .p12 格式并设置密码"
        echo ""
        echo "5. 转换为 base64:"
        echo "   base64 -i certificate.p12 -o certificate.txt"
        ;;
    Linux)
        echo ""
        echo "🐧 Linux 构建设置"
        echo "================"
        echo "Linux 平台通常不需要特殊的代码签名配置"
        echo "确保安装了必要的构建依赖:"
        echo "sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf"
        ;;
    *)
        echo ""
        echo "🪟 Windows 代码签名设置"
        echo "======================"
        echo "1. 从证书颁发机构购买代码签名证书"
        echo "2. 将证书导出为 .p12 格式"
        echo "3. 转换为 base64:"
        echo "   certutil -encode certificate.p12 certificate.txt"
        ;;
esac

# 环境变量设置提示
echo ""
echo "🔧 环境变量设置"
echo "==============="
echo "在你的 shell 配置文件中添加以下环境变量:"
echo ""
echo "# Tauri 更新器"
echo "export TAURI_PRIVATE_KEY=\"$(cat $CONFIG_DIR/rshield.key 2>/dev/null || echo 'YOUR_PRIVATE_KEY')\""
echo "export TAURI_KEY_PASSWORD=\"YOUR_KEY_PASSWORD\""
echo ""

if [ "$MACHINE" = "Mac" ]; then
    echo "# macOS 签名"
    echo "export APPLE_CERTIFICATE=\"YOUR_BASE64_CERTIFICATE\""
    echo "export APPLE_CERTIFICATE_PASSWORD=\"YOUR_CERTIFICATE_PASSWORD\""
    echo "export APPLE_SIGNING_IDENTITY=\"Developer ID Application: Your Name (TEAM_ID)\""
    echo "export APPLE_TEAM_ID=\"YOUR_TEAM_ID\""
    echo "export APPLE_ID=\"your-apple-id@example.com\""
    echo "export APPLE_PASSWORD=\"YOUR_APP_SPECIFIC_PASSWORD\""
elif [ "$MACHINE" != "Linux" ]; then
    echo "# Windows 签名"
    echo "export WINDOWS_CERTIFICATE=\"YOUR_BASE64_CERTIFICATE\""
    echo "export WINDOWS_CERTIFICATE_PASSWORD=\"YOUR_CERTIFICATE_PASSWORD\""
fi

echo ""
echo "📚 更多信息请参考: docs/code-signing.md"
echo "✅ 设置完成！"