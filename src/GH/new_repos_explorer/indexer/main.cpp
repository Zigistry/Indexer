#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <httplib.h>
#include <iostream>
#include <nlohmann/json.hpp>
#include <sstream>
#include <string>
#include <thread>
#include <vector>
#include <SQLiteCpp/SQLiteCpp.h>
#include <utilities.h>

struct Repo {
    std::string platform;
    std::string owner_name;
    std::string repo_name;
};

bool threat_detected(const Repo& repo);

void mark_in_database_for_manual_review(SQLite::Database& database, const Repo& repo);

void Index_repo(SQLite::Database& database, const Repo& repo);

Repo split_repo_id_into_parts(const std::string& id)
{
    Repo repo;
    std::stringstream ss(id);

    std::getline(ss, repo.platform, '/');
    std::getline(ss, repo.owner_name, '/');
    std::getline(ss, repo.repo_name);

    return repo;
}

int main()
{
    std::ifstream input_file("./tmp/github_every_repo_name_and_owner_name.txt");

    SQLite::Database db("./tmp/zigistry.db", SQLite::OPEN_READWRITE);

    SQLite::Statement check_repo_in_repos_query(db, "SELECT 1 FROM repos WHERE id = ? LIMIT 1");
    SQLite::Statement check_owner_in_banned_users_query(db, "SELECT 1 FROM banned_users WHERE id = ? LIMIT 1");

    std::vector<Repo> repos_to_index;

    char line[500];

    while (input_file.getline(line, sizeof(line))) {
        const auto raw_repo = "gh/" + std::string(line);
        const auto repo = split_repo_id_into_parts(raw_repo);

        check_repo_in_repos_query.bind(1, raw_repo);
        const bool repo_already_exists = check_repo_in_repos_query.executeStep();

        check_owner_in_banned_users_query.bind(1, repo.owner_name);
        const bool owner_is_banned = check_owner_in_banned_users_query.executeStep();

        if (repo_already_exists || owner_is_banned) {
            continue;
        }

        repos_to_index.push_back(repo);
    }

    #pragma omp parallel for num_threads(2)
    for (const Repo& repo : repos_to_index) {
        if (threat_detected(repo)) {
            mark_in_database_for_manual_review(db, repo);
        } else {
            Index_repo(db, repo);
        }
    }

    return 0;
}
