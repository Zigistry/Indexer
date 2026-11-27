const std = @import("std");

fn generate_base_module(b: *std.Build, target: std.Build.ResolvedTarget, optimize: std.builtin.Optimize) *std.Build.Module {
    const base_mod = b.createModule(.{
        .target = target,
        .optimize = optimize,
        .link_libcpp = true,
    });

    base_mod.addIncludePath(b.path("src/GH"));
    base_mod.addIncludePath(b.path("src/utilities"));

    if (target.result.os.tag == .macos) {
        base_mod.addIncludePath(.{ .cwd_relative = "/opt/homebrew/include" });
        base_mod.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/opt/sqlitecpp/lib" });
        base_mod.addIncludePath(.{ .cwd_relative = "/opt/homebrew/opt/libomp/include" });
        base_mod.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/opt/libomp/lib" });
    }

    base_mod.addCSourceFile(.{
        .file = b.path("src/utilities/utilities.cpp"),
        .flags = &.{
            "-std=c++23",
            "-fopenmp",
            "-DCPPHTTPLIB_OPENSSL_SUPPORT",
        },
        .language = .cpp,
    });

    base_mod.addCSourceFile(.{
        .file = b.path("src/GH/generate_time_ranges.cpp"),
        .flags = &.{
            "-std=c++23",
            "-fopenmp",
            "-DCPPHTTPLIB_OPENSSL_SUPPORT",
        },
        .language = .cpp,
    });

    base_mod.linkSystemLibrary("SQLiteCpp", .{});
    base_mod.linkSystemLibrary("sqlite3", .{});

    base_mod.linkSystemLibrary("ssl", .{});
    base_mod.linkSystemLibrary("crypto", .{});
    base_mod.linkSystemLibrary("omp", .{});
    base_mod.linkFramework("CoreFoundation", .{});
    base_mod.linkFramework("Security", .{});
    return base_mod;
}

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    {
        const mod = generate_base_module(b, target, optimize);

        mod.addCSourceFile(.{
            .file = b.path("src/GH/new_repos_explorer/generate_time_ranges_for_crawler.cpp"),
            .flags = &.{
                "-std=c++23",
                "-fopenmp",
                "-DCPPHTTPLIB_OPENSSL_SUPPORT",
            },
            .language = .cpp,
        });

        const exe = b.addExecutable(.{
            .name = "GH_generate_time_ranges_for_crawler",
            .root_module = mod,
        });
        b.installArtifact(exe);
    }
}
