/*
 * UnScanMe Windows GUI Installer (Win32 API, zero external CRT dependencies).
 * Compiled on Linux with clang --target=x86_64-pc-windows-msvc and linked with lld-link.
 */
#include <stdint.h>
#include <stddef.h>

typedef void *HANDLE;
typedef void *HWND;
typedef void *HINSTANCE;
typedef uint32_t DWORD;
typedef uint16_t WORD;
typedef int BOOL;
typedef const wchar_t *LPCWSTR;
typedef wchar_t *LPWSTR;

#define WINAPI __stdcall
#define INVALID_HANDLE_VALUE ((HANDLE)(intptr_t)-1)
#define GENERIC_WRITE 0x40000000u
#define CREATE_ALWAYS 2u
#define FILE_ATTRIBUTE_NORMAL 0x00000080u
#define INFINITE 0xFFFFFFFFu
#define HEAP_ZERO_MEMORY 0x00000008u

#define MB_OK 0x00000000u
#define MB_OKCANCEL 0x00000001u
#define MB_ICONERROR 0x00000010u
#define MB_ICONINFORMATION 0x00000040u
#define IDCANCEL 2

#define SEE_MASK_NOCLOSEPROCESS 0x00000040u
#define SW_HIDE 0

typedef struct {
    DWORD cbSize;
    DWORD fMask;
    HWND hwnd;
    LPCWSTR lpVerb;
    LPCWSTR lpFile;
    LPCWSTR lpParameters;
    LPCWSTR lpDirectory;
    int nShow;
    HINSTANCE hInstApp;
    void *lpIDList;
    LPCWSTR lpClass;
    void *hkeyClass;
    DWORD dwHotKey;
    HANDLE hIcon;
    HANDLE hProcess;
} SHELLEXECUTEINFOW;

__declspec(dllimport) HANDLE WINAPI GetProcessHeap(void);
__declspec(dllimport) void *WINAPI HeapAlloc(HANDLE hHeap, DWORD dwFlags, size_t dwBytes);
__declspec(dllimport) BOOL WINAPI HeapFree(HANDLE hHeap, DWORD dwFlags, void *lpMem);
__declspec(dllimport) DWORD WINAPI GetTempPathW(DWORD nBufferLength, LPWSTR lpBuffer);
__declspec(dllimport) HANDLE WINAPI CreateFileW(
    LPCWSTR lpFileName, DWORD dwDesiredAccess, DWORD dwShareMode,
    void *lpSecurityAttributes, DWORD dwCreationDisposition,
    DWORD dwFlagsAndAttributes, HANDLE hTemplateFile);
__declspec(dllimport) BOOL WINAPI WriteFile(
    HANDLE hFile, const void *lpBuffer, DWORD nNumberOfBytesToWrite,
    DWORD *lpNumberOfBytesWritten, void *lpOverlapped);
__declspec(dllimport) BOOL WINAPI CloseHandle(HANDLE hObject);
__declspec(dllimport) BOOL WINAPI DeleteFileW(LPCWSTR lpFileName);
__declspec(dllimport) DWORD WINAPI WaitForSingleObject(HANDLE hHandle, DWORD dwMilliseconds);
__declspec(dllimport) BOOL WINAPI GetExitCodeProcess(HANDLE hProcess, DWORD *lpExitCode);
__declspec(dllimport) void WINAPI ExitProcess(DWORD uExitCode);

__declspec(dllimport) int WINAPI MessageBoxW(HWND hWnd, LPCWSTR lpText, LPCWSTR lpCaption, unsigned int uType);
__declspec(dllimport) BOOL WINAPI ShellExecuteExW(SHELLEXECUTEINFOW *pExecInfo);

static void zmem(void *dst, size_t len) {
    unsigned char *p = (unsigned char *)dst;
    for (size_t i = 0; i < len; i++) {
        p[i] = 0;
    }
}

static size_t wlen(const wchar_t *s) {
    size_t n = 0;
    while (s[n] != 0) n++;
    return n;
}

static void wcpy(wchar_t *dst, const wchar_t *src) {
    while (*src != 0) {
        *dst++ = *src++;
    }
    *dst = 0;
}

static void wcat(wchar_t *dst, const wchar_t *src) {
    wcpy(dst + wlen(dst), src);
}

static const char PS_INSTALLER_SCRIPT[] =
    "$ErrorActionPreference = 'Stop'\r\n"
    "$ProgressPreference = 'SilentlyContinue'\r\n"
    "$InstallDir = Join-Path $env:LOCALAPPDATA 'Programs\\UnScanMe'\r\n"
    "New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null\r\n"
    "$ExePath = Join-Path $InstallDir 'unscanme.exe'\r\n"
    "$ReleaseApi = 'https://api.github.com/repos/J0seph-Mart1n/ImgToPdf-Rust/releases/latest'\r\n"
    "$Rel = Invoke-RestMethod -Uri $ReleaseApi -Headers @{ 'User-Agent' = 'UnScanMe-Setup' }\r\n"
    "$Installer = $Rel.assets | Where-Object { $_.name -match '(Setup.*\\.exe|\\.msi)$' } | Select-Object -First 1\r\n"
    "if ($Installer -and $Installer.name -match '\\.msi$') {\r\n"
    "  $MsiPath = Join-Path $env:TEMP $Installer.name\r\n"
    "  Invoke-WebRequest -Uri $Installer.browser_download_url -OutFile $MsiPath\r\n"
    "  Start-Process msiexec.exe -ArgumentList \"/i `\"$MsiPath`\" /passive\" -Wait\r\n"
    "} else {\r\n"
    "  $Bin = $Rel.assets | Where-Object { $_.name -match 'unscanme.*\\.exe$' -and $_.name -notmatch 'Setup' } | Select-Object -First 1\r\n"
    "  if (-not $Bin) { throw 'No Windows release binary found.' }\r\n"
    "  Invoke-WebRequest -Uri $Bin.browser_download_url -OutFile $ExePath\r\n"
    "}\r\n"
    "$Wsh = New-Object -ComObject WScript.Shell\r\n"
    "$StartDir = Join-Path $env:APPDATA 'Microsoft\\Windows\\Start Menu\\Programs'\r\n"
    "$DesktopDir = [Environment]::GetFolderPath('Desktop')\r\n"
    "foreach ($Dir in @($StartDir, $DesktopDir)) {\r\n"
    "  if ($Dir -and (Test-Path $Dir) -and (Test-Path $ExePath)) {\r\n"
    "    $Sc = $Wsh.CreateShortcut((Join-Path $Dir 'UnScanMe.lnk'))\r\n"
    "    $Sc.TargetPath = $ExePath\r\n"
    "    $Sc.WorkingDirectory = $InstallDir\r\n"
    "    $Sc.Description = 'UnScanMe — Local Document Workbench'\r\n"
    "    $Sc.Save()\r\n"
    "  }\r\n"
    "}\r\n"
    "$RegKey = 'HKCU:\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\UnScanMe'\r\n"
    "New-Item -Path $RegKey -Force | Out-Null\r\n"
    "Set-ItemProperty -Path $RegKey -Name DisplayName -Value 'UnScanMe'\r\n"
    "Set-ItemProperty -Path $RegKey -Name DisplayVersion -Value '0.1.0'\r\n"
    "Set-ItemProperty -Path $RegKey -Name Publisher -Value 'UnScanMe'\r\n"
    "Set-ItemProperty -Path $RegKey -Name InstallLocation -Value $InstallDir\r\n"
    "Set-ItemProperty -Path $RegKey -Name UninstallString -Value \"powershell.exe -NoProfile -Command `\"Remove-Item -Recurse -Force '$InstallDir'`\"\"\r\n"
    "if (Test-Path $ExePath) { Start-Process -FilePath $ExePath }\r\n";

void WINAPI WinMainCRTStartup(void) {
    const wchar_t *title = L"UnScanMe Setup (v0.1.0)";
    int choice = MessageBoxW(
        NULL,
        L"Install UnScanMe v0.1.0 on your computer?\n\n"
        L"UnScanMe will be installed to your Programs folder with Desktop and Start Menu shortcuts.",
        title,
        MB_OKCANCEL | MB_ICONINFORMATION
    );

    if (choice == IDCANCEL) {
        ExitProcess(0);
    }

    HANDLE heap = GetProcessHeap();
    wchar_t *tempDir = (wchar_t *)HeapAlloc(heap, HEAP_ZERO_MEMORY, 1024 * sizeof(wchar_t));
    wchar_t *scriptPath = (wchar_t *)HeapAlloc(heap, HEAP_ZERO_MEMORY, 1024 * sizeof(wchar_t));
    wchar_t *params = (wchar_t *)HeapAlloc(heap, HEAP_ZERO_MEMORY, 1280 * sizeof(wchar_t));
    if (!tempDir || !scriptPath || !params) {
        MessageBoxW(NULL, L"Memory allocation failed.", title, MB_OK | MB_ICONERROR);
        ExitProcess(1);
    }

    DWORD len = GetTempPathW(1000, tempDir);
    if (len == 0 || len >= 1000) {
        MessageBoxW(NULL, L"Unable to locate Windows %TEMP% directory.", title, MB_OK | MB_ICONERROR);
        ExitProcess(1);
    }

    wcpy(scriptPath, tempDir);
    wcat(scriptPath, L"unscanme_setup_0_1_0.ps1");

    HANDLE hFile = CreateFileW(
        scriptPath,
        GENERIC_WRITE,
        0,
        NULL,
        CREATE_ALWAYS,
        FILE_ATTRIBUTE_NORMAL,
        NULL
    );
    if (hFile == INVALID_HANDLE_VALUE) {
        MessageBoxW(NULL, L"Failed to create temporary installer script.", title, MB_OK | MB_ICONERROR);
        ExitProcess(1);
    }

    DWORD written = 0;
    DWORD scriptLen = (DWORD)(sizeof(PS_INSTALLER_SCRIPT) - 1);
    BOOL ok = WriteFile(hFile, PS_INSTALLER_SCRIPT, scriptLen, &written, NULL);
    CloseHandle(hFile);

    if (!ok || written != scriptLen) {
        DeleteFileW(scriptPath);
        MessageBoxW(NULL, L"Failed to write temporary installer script.", title, MB_OK | MB_ICONERROR);
        ExitProcess(1);
    }

    wcpy(params, L"-NoProfile -ExecutionPolicy Bypass -File \"");
    wcat(params, scriptPath);
    wcat(params, L"\"");

    SHELLEXECUTEINFOW sei;
    zmem(&sei, sizeof(sei));
    sei.cbSize = sizeof(sei);
    sei.fMask = SEE_MASK_NOCLOSEPROCESS;
    sei.lpVerb = L"open";
    sei.lpFile = L"powershell.exe";
    sei.lpParameters = params;
    sei.nShow = SW_HIDE;

    if (!ShellExecuteExW(&sei) || !sei.hProcess) {
        DeleteFileW(scriptPath);
        MessageBoxW(NULL, L"Could not launch PowerShell setup helper.", title, MB_OK | MB_ICONERROR);
        ExitProcess(1);
    }

    WaitForSingleObject(sei.hProcess, INFINITE);
    DWORD exitCode = 1;
    GetExitCodeProcess(sei.hProcess, &exitCode);
    CloseHandle(sei.hProcess);
    DeleteFileW(scriptPath);

    HeapFree(heap, 0, tempDir);
    HeapFree(heap, 0, scriptPath);
    HeapFree(heap, 0, params);

    if (exitCode == 0) {
        MessageBoxW(
            NULL,
            L"UnScanMe has been installed successfully!\n\n"
            L"You can launch UnScanMe from your Desktop or Start Menu.",
            title,
            MB_OK | MB_ICONINFORMATION
        );
    } else {
        MessageBoxW(
            NULL,
            L"Could not download the UnScanMe Windows release package.\n\n"
            L"Please check your internet connection or download the release directly from GitHub Releases.",
            title,
            MB_OK | MB_ICONERROR
        );
    }

    ExitProcess(exitCode);
}
