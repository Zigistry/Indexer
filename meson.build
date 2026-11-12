project(
    'Zigistry',
    'cpp',
    default_options: [
        'cpp_std=c++23',
    ],
)

add_project_arguments(
    '-Wall',
    '-Wextra',
    language: 'cpp',
)

run_target(
    'download_dependencies',
    command: [
        'sh',
        '-c',
        '''

        set -eu

        cd "$MESON_SOURCE_ROOT"

        mkdir -p libs/

        curl -fL https://sqlite.org/2026/sqlite-amalgamation-3540100.zip -o libs/sqlite.zip
        unzip -o libs/sqlite.zip -d libs
        rm -rf libs/sqlite
        mv libs/sqlite-amalgamation-3540100 libs/sqlite/
        rm -f libs/sqlite.zip

        curl -fL https://raw.githubusercontent.com/yhirose/cpp-httplib/refs/heads/master/httplib.h -o libs/httplib.h
        curl -fL https://github.com/nlohmann/json/releases/download/v3.12.0/json.hpp -o libs/json.hpp

    ''',
    ],
)

run_target(
    'format',
    command: [
        'sh',
        '-c',
        '''
      cd "$MESON_SOURCE_ROOT"

      clang-format -i --style=WebKit src/**/*.cpp
      clang-format -i --style=WebKit src/**/*.h
      clang-format -i --style=WebKit src/**/*.hpp

      meson format -i
    ''',
    ],
)

openssl = dependency('openssl', required: true)

apple_frameworks = dependency(
    'appleframeworks',
    modules: [
        'CoreFoundation',
        'Security',
    ],
)

executable(
    'GH_Indexer',
    'src/GH/indexer.cpp',
    include_directories: [
        include_directories('libs'),
    ],
    cpp_args: [
        '-I/opt/homebrew/opt/openssl@4/include',
    ],
    dependencies: [
        openssl,
        apple_frameworks,
    ],
    install: false,
)
