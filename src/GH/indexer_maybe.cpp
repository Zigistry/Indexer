// I think I should retry doing the indexing part.
// I have just added the docs.excalidraw in the docs.
// thats the logic behind this:
#include <iostream>
#include <random>
#define CPPHTTPLIB_OPENSSL_SUPPORT
#include <cstddef>
#include <cstdint>
#include <ctime>
#include <format>
#include <httplib.h>
#include <json.hpp>

struct Range {
    time_t low;
    time_t high;
    uint64_t repo_count;
};

const time_t month = 30ULL * 24ULL * 60ULL * 60ULL;
const time_t day = 24ULL * 60ULL * 60ULL;

const std::string repo_topic = "python";
std::string github_token;

uint64_t get_repo_count(const time_t low, const time_t high)
{
    const std::string query = R"""(

            query($search_query: String!) {
                search(
                    query: $search_query,
                    type: REPOSITORY,
                ) {
                    repositoryCount
                }
            }

    )""";

    char low_date[21], high_date[21];

    std::strftime(low_date, sizeof(low_date), "%Y-%m-%dT%H:%M:%SZ", std::gmtime(&low));
    std::strftime(high_date, sizeof(high_date), "%Y-%m-%dT%H:%M:%SZ", std::gmtime(&high));

    const std::string& search_query = std::format("topic:{} created:{}..{}", repo_topic, low_date, high_date);

    httplib::Client cli("https://api.github.com");

    while (1) {
        const auto response = cli.Post("/graphql",
            httplib::Headers {
                { "Authorization", "Bearer " + github_token },
                { "Accept", "application/vnd.github+json" },
                { "User-Agent", "Zigistry-Indexer" },
            },
            nlohmann::json {
                { "query", query },
                {
                    "variables",
                    { { "search_query", search_query } },
                },
            }
                .dump(),
            "application/json");

        if (!response)
            continue;

        if (response->status != 200)
            continue;

        const auto res = nlohmann::json::parse(response->body);

        int repo_count = res["data"]["search"]["repositoryCount"];
        std::cout << repo_count << std::endl;

        return repo_count;
    }
}

void generate_ranges(std::vector<Range>& ranges, const time_t low, const time_t high)
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

int main()
{
    const time_t current_epoch = time(NULL);
    const time_t twenty_ten = 1262304000;

    const char* gh_token = getenv("GH_API_KEY");

    if (gh_token == NULL) {
        std::cout << "GH_API_KEY not set." << std::endl;
        return 0;
    }

    github_token = gh_token;

    std::vector<Range> res;
    generate_ranges(res, twenty_ten, current_epoch);

    std::ofstream output_file("ranges.csv", std::ios::out);

    for (const Range &r : res) {
        output_file << r.low << "," << r.high << "," << r.repo_count << std::endl;
    }

    return 0;
}