#!/usr/bin/env bash
set -euo pipefail

REPO="Karben233/bili-hardcore"
BINARY="bili-hardcore"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# --- Helpers ---
info()  { printf '\033[1;34m[info]\033[0m  %s\n' "$*"; }
warn()  { printf '\033[1;33m[warn]\033[0m  %s\n' "$*"; }
error() { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "需要 $1 但未找到，请先安装"
  fi
}

# --- Preflight ---
need_cmd curl
need_cmd tar
need_cmd uname

# --- Detect OS ---
OS="$(uname -s)"
case "$OS" in
  Darwin) os="darwin" ;;
  Linux)  os="linux" ;;
  MINGW*|MSYS*|CYGWIN*) error "Windows 请直接下载 zip 文件: https://github.com/${REPO}/releases/latest" ;;
  *)      error "不支持的操作系统: $OS" ;;
esac

# --- Detect Arch ---
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)  arch="x64" ;;
  aarch64|arm64) arch="arm64" ;;
  *)             error "不支持的架构: $ARCH" ;;
esac

# --- Pick variant ---
if [ "$os" = "linux" ]; then
  # Prefer musl for static linking
  variant="-musl"
  info "Linux 检测到，优先使用 musl 静态链接版本"
else
  variant=""
fi

# --- Determine platform string ---
if [ "$os" = "darwin" ]; then
  # Universal binary covers both architectures
  PLATFORM="darwin-universal"
else
  PLATFORM="linux-${arch}${variant}"
fi

# --- Get latest version ---
info "正在获取最新版本..."
VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
if [ -z "$VERSION" ]; then
  error "无法获取最新版本号"
fi
info "最新版本: ${VERSION}"

# --- Download ---
if [ "$os" = "darwin" ]; then
  EXT="tar.gz"
else
  EXT="tar.gz"
fi

FILENAME="${BINARY}-${VERSION}-${PLATFORM}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILENAME}"

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "正在下载 ${FILENAME}..."
curl -fsSL --progress-bar -o "${TMPDIR}/${FILENAME}" "$URL" || error "下载失败: ${URL}"

# --- Install ---
info "正在解压..."
tar -xzf "${TMPDIR}/${FILENAME}" -C "${TMPDIR}"

mkdir -p "${INSTALL_DIR}"
mv "${TMPDIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"
chmod +x "${INSTALL_DIR}/${BINARY}"

# --- Done ---
info "安装完成: ${INSTALL_DIR}/${BINARY}"

if ! echo ":${PATH}:" | grep -q ":${INSTALL_DIR}:"; then
  warn "${INSTALL_DIR} 不在 PATH 中，请运行以下命令添加:"
  echo ""
  echo "  echo 'export PATH=\"\${HOME}/.local/bin:\$PATH\"' >> ~/.bashrc"
  echo "  source ~/.bashrc"
  echo ""
fi

info "运行 '${BINARY}' 开始使用, '${BINARY} --help' 了解更多命令"
