@echo off
chcp 65001 >nul
echo ========================================================
echo   QuizPulse Windows 一键编译与安装包构建脚本 (Tauri v2)
echo ========================================================
echo.

where node >nul 2>nul
if %errorlevel% neq 0 (
    echo [错误] 未检测到 Node.js，请先安装 Node.js LTS (https://nodejs.org/)
    pause
    exit /b 1
)

where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [错误] 未检测到 Rust/Cargo，请先安装 Rustup (https://rustup.rs/)
    pause
    exit /b 1
)

echo [1/3] 正在安装前端依赖 (npm install)...
call npm install
if %errorlevel% neq 0 (
    echo [错误] npm install 失败，请检查网络连接或镜像源。
    pause
    exit /b 1
)

echo.
echo [2/3] 正在编译并打包 Windows 安装程序 (NSIS .exe / .msi)...
echo       首次编译 Rust 依赖约需 2~5 分钟，后续增量编译仅需数秒。
call npm run tauri build
if %errorlevel% neq 0 (
    echo [错误] Tauri 编译失败。请确认已安装 Visual Studio C++ Build Tools (Desktop development with C++)。
    pause
    exit /b 1
)

echo.
echo [3/3] 打包成功！正在打开安装包输出目录...
explorer "src-tauri\target\release\bundle"
pause
