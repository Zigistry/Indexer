#include <chrono>
#include <cstdint>
#include <cstdlib>
#include <httplib.h>
#include <iostream>
#include <nlohmann/json.hpp>
#include <string>
#include <thread>

struct Repo {
    std::string platform;
    std::string owner_name;
    std::string repo_name;
};

const std::string fetch_repo_gql_query_fragment = R"""(
fragment RepoFields on Repository {
  name
  description
  pushedAt
  createdAt
  isArchived
  isDisabled
  isFork
  forkCount
  stargazerCount
  issues {
    totalCount
  }
  watchers {
    totalCount
  }
  defaultBranchRef {
    name
    target {
      ... on Commit {
        oid
      }
    }
  }
  licenseInfo {
    spdxId
  }
  primaryLanguage {
    name
  }
  owner {
    login
    ... on User {
      bio
    }
    ... on Organization {
      description
    }
  }
  repositoryTopics(first: 10) {
    nodes {
      topic {
        name
      }
    }
  }
  releases(first: 5, orderBy: { field: CREATED_AT, direction: DESC }) {
    nodes {
      tagName
      isPrerelease
      publishedAt
    }
  }
  languages(first: 10, orderBy: { field: SIZE, direction: DESC }) {
    nodes {
      name
    }
  }
}
)""";

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
