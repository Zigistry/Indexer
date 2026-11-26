const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const mod = b.createModule(.{
        .target = target,
        .optimize = optimize,
        .link_libcpp = true,
    });

    mod.addIncludePath(b.path("src/GH"));
    mod.addIncludePath(b.path("src/utilities"));
    mod.addIncludePath(.{ .cwd_relative = "/opt/homebrew/include" });

    mod.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/opt/sqlitecpp/lib" });

    mod.addIncludePath(.{ .cwd_relative = "/opt/homebrew/opt/libomp/include" });
    mod.addLibraryPath(.{ .cwd_relative = "/opt/homebrew/opt/libomp/lib" });

    mod.addCSourceFile(.{
        .file = b.path("src/GH/new_repos_explorer/generate_time_ranges_for_crawler.cpp"),
        .flags = &.{
            "-std=c++23",
            "-fopenmp",
            "-DCPPHTTPLIB_OPENSSL_SUPPORT",
        },
        .language = .cpp,
    });

    mod.addCSourceFile(.{
        .file = b.path("src/utilities/utilities.cpp"),
        .flags = &.{
            "-std=c++23",
            "-fopenmp",
            "-DCPPHTTPLIB_OPENSSL_SUPPORT",
        },
        .language = .cpp,
    });

    mod.addCSourceFile(.{
        .file = b.path("src/GH/generate_time_ranges.cpp"),
        .flags = &.{
            "-std=c++23",
            "-fopenmp",
            "-DCPPHTTPLIB_OPENSSL_SUPPORT",
        },
        .language = .cpp,
    });

    mod.linkSystemLibrary("SQLiteCpp", .{});
    mod.linkSystemLibrary("sqlite3", .{});

    mod.linkSystemLibrary("ssl", .{});
    mod.linkSystemLibrary("crypto", .{});
    mod.linkSystemLibrary("omp", .{});
    mod.linkFramework("CoreFoundation", .{});
    mod.linkFramework("Security", .{});

    const exe = b.addExecutable(.{
        .name = "generate_time_ranges_for_crawler",
        .root_module = mod,
    });

    b.installArtifact(exe);
}
