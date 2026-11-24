/*
 * i hate scans Native Linux Universal GUI & CLI Installer (x86_64 ELF).
 * Embeds the release binary, assets, SVG icon, and .desktop entry in .rodata.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

extern const unsigned char _ihs_payload_start[];
extern const unsigned char _ihs_payload_end[];

static int has_cmd(const char *cmd) {
    const char *path = getenv("PATH");
    if (!path) return 0;
    char *copy = strdup(path);
    if (!copy) return 0;
    int found = 0;
    for (char *dir = strtok(copy, ":"); dir; dir = strtok(NULL, ":")) {
        char full[PATH_MAX];
        snprintf(full, sizeof(full), "%s/%s", dir, cmd);
        if (access(full, X_OK) == 0) {
            found = 1;
            break;
        }
    }
    free(copy);
    return found;
}

static int run_cmd(char *const argv[]) {
    pid_t pid = fork();
    if (pid < 0) return -1;
    if (pid == 0) {
        execvp(argv[0], argv);
        _exit(127);
    }
    int status = 0;
    if (waitpid(pid, &status, 0) < 0) return -1;
    return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
}

static int run_cmd_quiet(char *const argv[]) {
    pid_t pid = fork();
    if (pid < 0) return -1;
    if (pid == 0) {
        int devnull = open("/dev/null", O_WRONLY);
        if (devnull >= 0) {
            dup2(devnull, STDOUT_FILENO);
            dup2(devnull, STDERR_FILENO);
            close(devnull);
        }
        execvp(argv[0], argv);
        _exit(127);
    }
    int status = 0;
    if (waitpid(pid, &status, 0) < 0) return -1;
    return WIFEXITED(status) ? WEXITSTATUS(status) : -1;
}

static int mkdir_p(const char *path, mode_t mode) {
    char tmp[PATH_MAX];
    snprintf(tmp, sizeof(tmp), "%s", path);
    size_t len = strlen(tmp);
    if (len == 0) return -1;
    if (tmp[len - 1] == '/') tmp[len - 1] = '\0';
    for (char *p = tmp + 1; *p; p++) {
        if (*p == '/') {
            *p = '\0';
            if (mkdir(tmp, mode) != 0 && errno != EEXIST) return -1;
            *p = '/';
        }
    }
    if (mkdir(tmp, mode) != 0 && errno != EEXIST) return -1;
    return 0;
}

static void gui_notify(const char *msg, int is_error) {
    if (has_cmd("zenity")) {
        char text_arg[512];
        snprintf(text_arg, sizeof(text_arg), "--text=%s", msg);
        char *args[] = {
            "zenity",
            is_error ? "--error" : "--info",
            "--title=i hate scans Installer v0.1.0",
            text_arg,
            NULL
        };
        run_cmd_quiet(args);
    } else if (has_cmd("kdialog")) {
        char *args[] = {
            "kdialog",
            is_error ? "--error" : "--msgbox",
            (char *)msg,
            "--title",
            "i hate scans Installer v0.1.0",
            NULL
        };
        run_cmd_quiet(args);
    } else if (has_cmd("notify-send")) {
        char *args[] = {"notify-send", "i hate scans Installer", (char *)msg, NULL};
        run_cmd_quiet(args);
    }
}

static int gui_confirm(void) {
    if (has_cmd("zenity")) {
        char *args[] = {
            "zenity",
            "--question",
            "--title=i hate scans Installer v0.1.0",
            "--text=Install i hate scans on your computer?",
            "--ok-label=Install",
            NULL
        };
        return run_cmd_quiet(args) == 0;
    }
    if (has_cmd("kdialog")) {
        char *args[] = {
            "kdialog",
            "--yesno",
            "Install i hate scans on your computer?",
            "--title",
            "i hate scans Installer v0.1.0",
            NULL
        };
        return run_cmd_quiet(args) == 0;
    }
    return 1;
}

static int write_entire_file(const char *path, const void *buf, size_t len, mode_t mode) {
    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, mode);
    if (fd < 0) return -1;
    const unsigned char *p = (const unsigned char *)buf;
    size_t rem = len;
    while (rem > 0) {
        ssize_t n = write(fd, p, rem);
        if (n <= 0) {
            if (errno == EINTR) continue;
            close(fd);
            return -1;
        }
        p += n;
        rem -= (size_t)n;
    }
    close(fd);
    return chmod(path, mode);
}

static void launch_detached(const char *exe_path) {
    pid_t pid = fork();
    if (pid == 0) {
        setsid();
        int devnull = open("/dev/null", O_RDWR);
        if (devnull >= 0) {
            dup2(devnull, STDIN_FILENO);
            dup2(devnull, STDOUT_FILENO);
            dup2(devnull, STDERR_FILENO);
            close(devnull);
        }
        char *const args[] = {(char *)exe_path, NULL};
        execv(exe_path, args);
        _exit(127);
    }
}

int main(int argc, char **argv) {
    int system_mode = 0;
    int uninstall = 0;
    int quiet = 0;
    int no_launch = 0;
    int update_desktop = 1;

    for (int i = 1; i < argc; i++) {
        if (strcmp(argv[i], "--system") == 0) system_mode = 1;
        else if (strcmp(argv[i], "--uninstall") == 0) uninstall = 1;
        else if (strcmp(argv[i], "--quiet") == 0) quiet = 1;
        else if (strcmp(argv[i], "--no-launch") == 0) no_launch = 1;
        else if (strcmp(argv[i], "--no-desktop-update") == 0) update_desktop = 0;
        else if (strcmp(argv[i], "-h") == 0 || strcmp(argv[i], "--help") == 0) {
            printf("i hate scans 0.1.0 Universal Linux Installer\n"
                   "Usage: %s [--user | --system | --uninstall | --quiet | --no-launch]\n", argv[0]);
            return 0;
        }
    }

    if (system_mode && geteuid() != 0) {
        if (has_cmd("pkexec")) {
            char self[PATH_MAX];
            ssize_t n = readlink("/proc/self/exe", self, sizeof(self) - 1);
            if (n > 0) {
                self[n] = '\0';
                char *args[] = {"pkexec", self, "--system", "--quiet", NULL};
                return run_cmd(args);
            }
        }
        fprintf(stderr, "Run with sudo for --system install, or omit --system to install for current user.\n");
        return 1;
    }

    const char *home = getenv("HOME");
    if (!home || !*home) home = "/tmp";
    const char *xdg_data = getenv("XDG_DATA_HOME");

    char bin_dir[PATH_MAX], share_dir[PATH_MAX], lib_dir[PATH_MAX];
    if (system_mode) {
        snprintf(bin_dir, sizeof(bin_dir), "/usr/local/bin");
        snprintf(share_dir, sizeof(share_dir), "/usr/local/share");
        snprintf(lib_dir, sizeof(lib_dir), "/usr/local/lib/i-hate-scans");
    } else {
        snprintf(bin_dir, sizeof(bin_dir), "%s/.local/bin", home);
        if (xdg_data && *xdg_data) snprintf(share_dir, sizeof(share_dir), "%s", xdg_data);
        else snprintf(share_dir, sizeof(share_dir), "%s/.local/share", home);
        snprintf(lib_dir, sizeof(lib_dir), "%s/i-hate-scans", share_dir);
    }

    char exe_dest[PATH_MAX], bin_link[PATH_MAX], uninstall_bin[PATH_MAX];
    char apps_dir[PATH_MAX], icons_dir[PATH_MAX], desktop_dest[PATH_MAX], icon_dest[PATH_MAX];
    snprintf(exe_dest, sizeof(exe_dest), "%s/i-hate-scans", lib_dir);
    snprintf(bin_link, sizeof(bin_link), "%s/i-hate-scans", bin_dir);
    snprintf(uninstall_bin, sizeof(uninstall_bin), "%s/i-hate-scans-uninstall", bin_dir);
    snprintf(apps_dir, sizeof(apps_dir), "%s/applications", share_dir);
    snprintf(icons_dir, sizeof(icons_dir), "%s/icons/hicolor/scalable/apps", share_dir);
    snprintf(desktop_dest, sizeof(desktop_dest), "%s/i-hate-scans.desktop", apps_dir);
    snprintf(icon_dest, sizeof(icon_dest), "%s/i-hate-scans.svg", icons_dir);

    if (uninstall) {
        char *rm_args[] = {"rm", "-rf", lib_dir, bin_link, uninstall_bin, desktop_dest, icon_dest, NULL};
        run_cmd(rm_args);
        printf("i hate scans uninstalled from %s\n", lib_dir);
        return 0;
    }

    int gui_session = !quiet && !isatty(STDOUT_FILENO) &&
                      (getenv("DISPLAY") != NULL || getenv("WAYLAND_DISPLAY") != NULL);
    if (gui_session && !gui_confirm()) {
        return 0;
    }

    char tmp_template[] = "/tmp/ihatescans_install_XXXXXX";
    char *tmp_dir = mkdtemp(tmp_template);
    if (!tmp_dir) {
        if (gui_session) gui_notify("Failed to create temporary extraction directory.", 1);
        perror("mkdtemp");
        return 1;
    }

    char payload_gz[PATH_MAX];
    snprintf(payload_gz, sizeof(payload_gz), "%s/payload.tar.gz", tmp_dir);
    size_t payload_size = (size_t)(_ihs_payload_end - _ihs_payload_start);
    if (write_entire_file(payload_gz, _ihs_payload_start, payload_size, 0600) != 0) {
        if (gui_session) gui_notify("Failed to extract i hate scans payload.", 1);
        return 1;
    }

    char *tar_args[] = {"tar", "-xzf", payload_gz, "-C", tmp_dir, NULL};
    if (run_cmd_quiet(tar_args) != 0) {
        if (gui_session) gui_notify("Failed to unpack i hate scans archive (tar).", 1);
        fprintf(stderr, "Error: failed to extract embedded tar archive.\n");
        return 1;
    }

    mkdir_p(lib_dir, 0755);
    mkdir_p(bin_dir, 0755);
    mkdir_p(apps_dir, 0755);
    mkdir_p(icons_dir, 0755);

    char src_bin[PATH_MAX], src_assets[PATH_MAX], src_svg[PATH_MAX], dst_assets[PATH_MAX];
    snprintf(src_bin, sizeof(src_bin), "%s/i-hate-scans", tmp_dir);
    snprintf(src_assets, sizeof(src_assets), "%s/assets", tmp_dir);
    snprintf(src_svg, sizeof(src_svg), "%s/i-hate-scans.svg", tmp_dir);
    snprintf(dst_assets, sizeof(dst_assets), "%s/assets", lib_dir);

    char *cp_bin[] = {"install", "-m", "755", src_bin, exe_dest, NULL};
    if (run_cmd(cp_bin) != 0) {
        if (gui_session) gui_notify("Failed to install i hate scans binary.", 1);
        return 1;
    }
    char *rm_assets[] = {"rm", "-rf", dst_assets, NULL};
    run_cmd(rm_assets);
    if (access(src_assets, F_OK) == 0) {
        char *cp_assets[] = {"cp", "-r", src_assets, dst_assets, NULL};
        run_cmd(cp_assets);
    }

    unlink(bin_link);
    if (symlink(exe_dest, bin_link) != 0) perror("symlink i-hate-scans");

    char *cp_svg[] = {"install", "-m", "644", src_svg, icon_dest, NULL};
    run_cmd(cp_svg);

    char desktop_body[2048];
    snprintf(
        desktop_body, sizeof(desktop_body),
        "[Desktop Entry]\n"
        "Type=Application\n"
        "Name=i hate scans\n"
        "GenericName=Local Document Workbench\n"
        "Comment=Convert document images, prompts, and PDFs into editable HTML5 and vector PDFs locally\n"
        "Exec=\"%s\" %%f\n"
        "Icon=%s\n"
        "Terminal=false\n"
        "Categories=Office;Graphics;Utility;\n"
        "MimeType=image/png;image/jpeg;image/webp;image/bmp;image/tiff;application/pdf;\n"
        "StartupWMClass=i-hate-scans\n",
        bin_link, icon_dest
    );
    write_entire_file(desktop_dest, desktop_body, strlen(desktop_body), 0644);

    char uninstall_body[2048];
    snprintf(
        uninstall_body, sizeof(uninstall_body),
        "#!/usr/bin/env bash\n"
        "set -euo pipefail\n"
        "rm -rf \"%s\" \"%s\" \"%s\" \"%s\" \"%s\"\n"
        "echo \"i hate scans uninstalled.\"\n",
        lib_dir, bin_link, uninstall_bin, desktop_dest, icon_dest
    );
    write_entire_file(uninstall_bin, uninstall_body, strlen(uninstall_body), 0755);

    char *rm_tmp[] = {"rm", "-rf", tmp_dir, NULL};
    run_cmd(rm_tmp);

    if (update_desktop) {
        if (has_cmd("update-desktop-database")) {
            char *args[] = {"update-desktop-database", apps_dir, NULL};
            run_cmd_quiet(args);
        }
        if (has_cmd("gtk-update-icon-cache")) {
            char icons_root[PATH_MAX];
            snprintf(icons_root, sizeof(icons_root), "%s/icons/hicolor", share_dir);
            char *args[] = {"gtk-update-icon-cache", "-f", "-t", icons_root, NULL};
            run_cmd_quiet(args);
        }
    }

    printf("i hate scans installed!\n"
           "  Binary:      %s\n"
           "  Launcher:    %s\n"
           "  Uninstaller: %s\n",
           bin_link, desktop_dest, uninstall_bin);

    if (gui_session) {
        gui_notify("i hate scans installed successfully!\n\nFind it in your Applications menu or run `i-hate-scans`.", 0);
    }

    if (!no_launch) {
        launch_detached(bin_link);
    }

    return 0;
}
