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
    'format',
    command: [
        'sh',
        '-c',
        '''
        cd "$MESON_SOURCE_ROOT"

        clang-format -i --style='{BasedOnStyle: WebKit, IndentPPDirectives: Leave}' src/**/*.cpp
        clang-format -i --style='{BasedOnStyle: WebKit, IndentPPDirectives: Leave}' src/**/*.h
        clang-format -i --style='{BasedOnStyle: WebKit, IndentPPDirectives: Leave}' src/**/*.hpp

        meson format -i
        ''',
    ],
)

openssl = dependency('openssl', required: true)
openmp = dependency('openmp', language: 'cpp', required: true)

sqlite3 = dependency(
    'sqlite3',
    version: '>=3.38.0',
    required: true,
)

sqlitecpp = dependency(
    'SQLiteCpp',
    method: 'cmake',
    modules: ['SQLiteCpp'],
    required: true,
)

httplib = dependency(
    'httplib',
    method: 'cmake',
    modules: ['httplib::httplib'],
    required: true,
)

json = dependency('nlohmann_json', required: true)

platform_dependencies = [
    openssl,
    openmp,
    httplib,
    json,
]

if host_machine.system() == 'darwin'
    platform_dependencies += dependency(
        'appleframeworks',
        modules: [
            'CoreFoundation',
            'CFNetwork',
            'Security',
        ],
    )
endif

executable(
    'GH_Range_Gen_for_Indexer',
    'src/GH/range_generator_for_indexer.cpp',
    'src/utilities/utilities.cpp',
    include_directories: [
        include_directories('src/utilities'),
    ],
    dependencies: platform_dependencies,
    install: false,
)

executable(
    'GH_Indexer',
    'src/GH/indexer.cpp',
    'src/utilities/utilities.cpp',
    include_directories: [
        include_directories('src/utilities'),
    ],
    dependencies: platform_dependencies + [
        sqlite3,
        sqlitecpp,
    ],
    install: false,
)
