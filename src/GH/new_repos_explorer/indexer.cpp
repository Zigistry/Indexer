#include <algorithm>
#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <fstream>
#include <httplib.h>
#include <iostream>
#include <nlohmann/json.hpp>
#include <string>
#include <thread>
#include <SQLiteCpp/SQLiteCpp.h>
#include <utilities.h>
#include <vector>
#include "fetch_repo_fragment.hh"

struct Repo {
    std::string platform;
    std::string owner_name;
    std::string repo_name;
};


Repo split_repo_id_into_parts(const std::string& id)
{
    Repo repo;
    std::stringstream ss(id);

    std::getline(ss, repo.platform, '/');
    std::getline(ss, repo.owner_name, '/');
    std::getline(ss, repo.repo_name);

    return repo;
}



nlohmann::json fetch_repository_from_github(const Repo& repo)
{
    const std::string repository_id = repo.platform + "/" + repo.owner_name + "/" + repo.repo_name;

    const char* github_api_token = std::getenv("GH_API_KEY");
    if (github_api_token == nullptr) {
        std::cerr << "GH_API_KEY not set." << std::endl;
        return nlohmann::json::object();
    }

    const std::string graphql_query = R"""(
    query IndexRepository($owner: String!, $name: String!) {
        repository(owner: $owner, name: $name) {
            ...RepoFields
        }
    }
    )""" + fetch_repo_gql_query_fragment;

    const nlohmann::json graphql_request_body = {
        { "query", graphql_query },
        {
            "variables",
            {
                { "owner", repo.owner_name },
                { "name", repo.repo_name },
            },
        },
    };

    thread_local httplib::Client github_api_client("https://api.github.com");
    github_api_client.set_connection_timeout(10, 0);
    github_api_client.set_read_timeout(10, 0);
    github_api_client.set_keep_alive(true);

    uint8_t retry_amount = 0;

    while (retry_amount < 5) {
        const auto response = github_api_client.Post("/graphql",
            httplib::Headers {
                { "Authorization", "Bearer " + std::string(github_api_token) },
                { "Accept", "application/vnd.github+json" },
                { "User-Agent", "Zigistry-Indexer" },
            },
            graphql_request_body.dump(), "application/json");

        if (!response) {
            std::cerr << "No response for " << repository_id << ", waiting 50 seconds."
                      << std::endl;
            std::this_thread::sleep_for(std::chrono::seconds(50));
            retry_amount++;
            continue;
        }

        if (response->status == 200) {
            const auto response_as_json = nlohmann::json::parse(response->body, nullptr, false);

            if (response_as_json.is_discarded() || response_as_json.contains("errors")) {
                std::cerr << "bad response for " << repository_id
                          << ", waiting 500s" << std::endl;
                std::cerr << response->body << std::endl;
                std::this_thread::sleep_for(std::chrono::seconds(500));
                retry_amount++;
                continue;
            }

            return response_as_json;
        }

        retry_amount++;
        std::cerr << "HTTP " << response->status << " for " << repository_id << std::endl;

        switch (response->status) {
        case 403:
            std::this_thread::sleep_for(std::chrono::seconds(500));
            continue;
        case 500:
        case 502:
        case 503:
        case 504:
            std::this_thread::sleep_for(std::chrono::seconds(2));
            continue;
        case 400:
        case 401:
        case 404:
            return nlohmann::json::object();
        default:
            std::this_thread::sleep_for(std::chrono::seconds(5));
            continue;
        }
    }

    std::cerr << "Skipping " << repository_id << " after 5 retries." << std::endl;
    return nlohmann::json::object();
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
        const nlohmann::json fetched_repository_json = fetch_repository_from_github(repo);
    }

    return 0;
}
