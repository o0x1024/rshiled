#!/bin/bash

# 代码签名验证脚本
# 用于验证代码签名配置是否正确

set -e

echo "🔍 rshield 代码签名验证脚本"
echo "============================"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查函数
check_env_var() {
    local var_name=$1
    local var_value=${!var_name}
    
    if [ -n "$var_value" ]; then
        echo -e "${GREEN}✅ $var_name${NC}: 已设置"
        return 0
    else
        echo -e "${RED}❌ $var_name${NC}: 未设置"
        return 1
    fi
}

check_optional_env_var() {
    local var_name=$1
    local var_value=${!var_name}
    
    if [ -n "$var_value" ]; then
        echo -e "${GREEN}✅ $var_name${NC}: 已设置"
    else
        echo -e "${YELLOW}⚠️  $var_name${NC}: 未设置 (可选)"
    fi
}

# 检测操作系统
OS="$(uname -s)"
case "${OS}" in
    Linux*)     MACHINE=Linux;;
    Darwin*)    MACHINE=Mac;;
    CYGWIN*)    MACHINE=Cygwin;;
    MINGW*)     MACHINE=MinGw;;
    *)          MACHINE="UNKNOWN:${OS}"
esac

echo "操作系统: ${MACHINE}"
echo ""

# 检查 Tauri CLI
echo "🔧 检查工具依赖"
echo "==============="
if command -v tauri &> /dev/null; then
    TAURI_VERSION=$(tauri --version)
    echo -e "${GREEN}✅ Tauri CLI${NC}: $TAURI_VERSION"
else
    echo -e "${RED}❌ Tauri CLI${NC}: 未安装"
    echo "   安装命令: cargo install tauri-cli"
fi

if command -v cargo &> /dev/null; then
    CARGO_VERSION=$(cargo --version)
    echo -e "${GREEN}✅ Cargo${NC}: $CARGO_VERSION"
else
    echo -e "${RED}❌ Cargo${NC}: 未安装"
fi

if command -v node &> /dev/null; then
    NODE_VERSION=$(node --version)
    echo -e "${GREEN}✅ Node.js${NC}: $NODE_VERSION"
else
    echo -e "${RED}❌ Node.js${NC}: 未安装"
fi

echo ""

# 检查通用环境变量
echo "🔐 检查 Tauri 更新器配置"
echo "========================"
TAURI_ERRORS=0

if ! check_env_var "TAURI_PRIVATE_KEY"; then
    ((TAURI_ERRORS++))
fi

if ! check_env_var "TAURI_KEY_PASSWORD"; then
    ((TAURI_ERRORS++))
fi

# 验证私钥格式
if [ -n "$TAURI_PRIVATE_KEY" ]; then
    if echo "$TAURI_PRIVATE_KEY" | grep -q "BEGIN PRIVATE KEY"; then
        echo -e "${GREEN}✅ TAURI_PRIVATE_KEY 格式${NC}: 正确"
    else
        echo -e "${RED}❌ TAURI_PRIVATE_KEY 格式${NC}: 可能不正确"
        ((TAURI_ERRORS++))
    fi
fi

echo ""

# 平台特定检查
case "${MACHINE}" in
    Mac)
        echo "🍎 检查 macOS 代码签名配置"
        echo "=========================="
        MACOS_ERRORS=0
        
        if ! check_env_var "APPLE_CERTIFICATE"; then
            ((MACOS_ERRORS++))
        fi
        
        if ! check_env_var "APPLE_CERTIFICATE_PASSWORD"; then
            ((MACOS_ERRORS++))
        fi
        
        if ! check_env_var "APPLE_SIGNING_IDENTITY"; then
            ((MACOS_ERRORS++))
        fi
        
        if ! check_env_var "APPLE_TEAM_ID"; then
            ((MACOS_ERRORS++))
        fi
        
        check_optional_env_var "APPLE_ID"
        check_optional_env_var "APPLE_PASSWORD"
        
        # 检查本地签名身份
        echo ""
        echo "🔍 检查本地签名身份"
        if command -v security &> /dev/null; then
            IDENTITIES=$(security find-identity -v -p codesigning 2>/dev/null | grep "Developer ID Application" | wc -l)
            if [ "$IDENTITIES" -gt 0 ]; then
                echo -e "${GREEN}✅ 找到 $IDENTITIES 个 Developer ID Application 证书${NC}"
                security find-identity -v -p codesigning | grep "Developer ID Application"
            else
                echo -e "${RED}❌ 未找到 Developer ID Application 证书${NC}"
                ((MACOS_ERRORS++))
            fi
        fi
        
        if [ $MACOS_ERRORS -eq 0 ]; then
            echo -e "\n${GREEN}✅ macOS 代码签名配置完整${NC}"
        else
            echo -e "\n${RED}❌ macOS 代码签名配置不完整 ($MACOS_ERRORS 个错误)${NC}"
        fi
        ;;
    Linux)
        echo "🐧 Linux 平台检查"
        echo "================"
        echo -e "${GREEN}✅ Linux 平台无需特殊代码签名配置${NC}"
        
        # 检查构建依赖
        echo ""
        echo "🔍 检查构建依赖"
        DEPS=("pkg-config" "libgtk-3-dev" "libwebkit2gtk-4.0-dev")
        for dep in "${DEPS[@]}"; do
            if dpkg -l | grep -q "$dep"; then
                echo -e "${GREEN}✅ $dep${NC}: 已安装"
            else
                echo -e "${YELLOW}⚠️  $dep${NC}: 未安装"
            fi
        done
        ;;
    *)
        echo "🪟 检查 Windows 代码签名配置"
        echo "==========================="
        WINDOWS_ERRORS=0
        
        if ! check_env_var "WINDOWS_CERTIFICATE"; then
            ((WINDOWS_ERRORS++))
        fi
        
        if ! check_env_var "WINDOWS_CERTIFICATE_PASSWORD"; then
            ((WINDOWS_ERRORS++))
        fi
        
        if [ $WINDOWS_ERRORS -eq 0 ]; then
            echo -e "\n${GREEN}✅ Windows 代码签名配置完整${NC}"
        else
            echo -e "\n${RED}❌ Windows 代码签名配置不完整 ($WINDOWS_ERRORS 个错误)${NC}"
        fi
        ;;
esac

echo ""

# 检查项目配置
echo "📁 检查项目配置"
echo "==============="
if [ -f "tauri.conf.json" ]; then
    echo -e "${GREEN}✅ tauri.conf.json${NC}: 存在"
    
    # 检查签名相关配置
    if grep -q '"macOS"' tauri.conf.json; then
        echo -e "${GREEN}✅ macOS 配置${NC}: 已配置"
    else
        echo -e "${YELLOW}⚠️  macOS 配置${NC}: 未配置"
    fi
    
    if grep -q '"windows"' tauri.conf.json; then
        echo -e "${GREEN}✅ Windows 配置${NC}: 已配置"
    else
        echo -e "${YELLOW}⚠️  Windows 配置${NC}: 未配置"
    fi
else
    echo -e "${RED}❌ tauri.conf.json${NC}: 不存在"
fi

if [ -f ".github/workflows/build.yml" ]; then
    echo -e "${GREEN}✅ GitHub Actions 工作流${NC}: 已配置"
else
    echo -e "${YELLOW}⚠️  GitHub Actions 工作流${NC}: 未配置"
fi

echo ""

# 总结
echo "📊 验证总结"
echo "=========="
TOTAL_ERRORS=$((TAURI_ERRORS + ${MACOS_ERRORS:-0} + ${WINDOWS_ERRORS:-0}))

if [ $TOTAL_ERRORS -eq 0 ]; then
    echo -e "${GREEN}🎉 所有配置检查通过！可以开始构建签名应用。${NC}"
else
    echo -e "${RED}⚠️  发现 $TOTAL_ERRORS 个配置问题，请参考 docs/code-signing.md 进行修复。${NC}"
fi

echo ""
echo "💡 提示:"
echo "- 运行 'scripts/setup-signing.sh' 来设置代码签名"
echo "- 查看 'docs/code-signing.md' 获取详细配置指南"
echo "- 使用 'yarn tauri build' 进行本地构建测试"