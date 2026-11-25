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

gh_include_directories = include_directories('src/GH')
utilities_include_directories = include_directories('src/utilities')

common_gh_sources = [
    'src/GH/generate_time_ranges.cpp',
    'src/utilities/utilities.cpp',
]

executable(
    'GH_Range_Gen_for_Crawler',
    'src/GH/new_repos_explorer/generate_time_ranges_for_crawler.cpp',
    common_gh_sources,
    include_directories: [
        gh_include_directories,
        utilities_include_directories,
    ],
    dependencies: platform_dependencies,
    install: false,
)

executable(
    'GH_Crawler',
    'src/GH/new_repos_explorer/crawler.cpp',
    'src/utilities/utilities.cpp',
    include_directories: utilities_include_directories,
    dependencies: platform_dependencies,
    install: false,
)

executable(
    'GH_Indexer',
    'src/GH/new_repos_explorer/indexer.cpp',
    'src/utilities/utilities.cpp',
    include_directories: utilities_include_directories,
    dependencies: platform_dependencies + [
        sqlite3,
        sqlitecpp,
    ],
    install: false,
)

executable(
    'GH_Range_Gen_for_Updater',
    'src/GH/existing_repos_updater/generate_time_ranges_for_updater.cpp',
    common_gh_sources,
    include_directories: [
        gh_include_directories,
        utilities_include_directories,
    ],
    dependencies: platform_dependencies,
    install: false,
)

executable(
    'Keyword_Extractor',
    'src/plugins/keyword_extractor/main.cpp',
    install: false,
)
