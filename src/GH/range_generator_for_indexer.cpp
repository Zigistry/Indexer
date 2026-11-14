/*
 * Copyright (c) Rohan Vashisht 2026.
 * Please check LICENSE.txt and LICENSE-ADDITIONAL.txt for
 * further details.
 *
 * This file will break a given timeline into
 * multiple ranges which contain < 1000 repositories.
 *
 * Because GitHub can only return < 1000 repositories
 * per query, breaking the timeline like this helps
 * with easy indexing.
 */

#define CPPHTTPLIB_OPENSSL_SUPPORT

#include <cstdint>
#include <ctime>
#include <format>
#include <httplib.h>
#include <iostream>
#include <json.hpp>
#include <utilities.h>

const httplib::Client client("https://api.github.com");

std::string github_token;
std::string repo_topic;

uint64_t get_repo_count(const time_t low, const time_t high)
{
    const std::string get_repo_count_query = R"""(

            query($search_query: String!) {
                search(
                    query: $search_query,
                    type: REPOSITORY,
                ) {
                    repositoryCount
                }
            }

    )""";

    const std::string& search_query = std::format(
        "topic:{} created:{}..{}", repo_topic, epoch_to_timestamp_string(low),
        epoch_to_timestamp_string(high));

    uint8_t retry_amount = 0;

    while (retry_amount < 5) {
        const auto response = client.Post("/graphql",
            httplib::Headers {
                { "Authorization", "Bearer " + github_token },
                { "Accept", "application/vnd.github+json" },
                { "User-Agent", "Zigistry-Indexer" },
            },
            nlohmann::json {
                { "query", get_repo_count_query },
                {
                    "variables",
                    { { "search_query", search_query } },
                },
            }
                .dump(),
            "application/json");

        if (!response) {
            std::cout << "Program is waiting for 500 seconds because no responce was "
                         "recieved."
                      << std::endl;
            sleep(500);
            retry_amount++;
            continue;
        }

        if (response->status != 200) {
            std::cout << "Program is waiting for 500 seconds because a not 200 "
                         "reponse was recieved."
                      << std::endl;
            sleep(500);
            retry_amount++;
            continue;
        }

        retry_amount = 0;

        const auto responce_as_json = nlohmann::json::parse(response->body);

        const uint64_t repo_count = responce_as_json["data"]["search"]["repositoryCount"];

        return repo_count;
    }

    std::cout << "Exiting after 5 retries." << std::endl;

    std::exit(1);
}

void generate_ranges(std::vector<Range>& ranges, const time_t low,
    const time_t high)
{
    const uint64_t repo_count = get_repo_count(low, high);
    if (low >= high) {
        return;
    }

    if (repo_count < 1000) {
        ranges.push_back({ low, high, repo_count });
    } else {
        const time_t mid = low + (high - low) / 2;
        generate_ranges(ranges, low, mid);
        generate_ranges(ranges, mid + 1, high);
    }
}

int main(int argc, char* argv[])
{
    if (argc != 2) {
        std::cerr << "Please provide topic name" << std::endl;
    }

    repo_topic = argv[1];
    const time_t current_epoch = time(nullptr);

    // I was 2 years old back then XD.
    const time_t epoch_2008_march = 1204329600;

    const char* gh_token = getenv("GH_API_KEY");

    if (gh_token == nullptr) {
        std::cout << "GH_API_KEY not set." << std::endl;
        return 0;
    }

    github_token = gh_token;

    std::vector<Range> time_ranges;
    generate_ranges(time_ranges, epoch_2008_march, current_epoch);

    std::ofstream output_file("./tmp/github_ranges.csv", std::ios::out);

    for (const Range& time_range : time_ranges) {
        output_file << time_range.low << "," << time_range.high << ","
                    << time_range.repo_count << '\n';
    }

    return 0;
}
