#define CPPHTTPLIB_OPENSSL_SUPPORT
#include <cstddef>
#include <cstdint>
#include <ctime>
#include <format>
#include <httplib.h>
#include <iostream>
#include <json.hpp>

// Using python as topic because
// it has been around since 2008
// and is a very common topic.
// best to stress test this.
const std::string repo_topic = "python";
const time_t SECONDS_IN_MONTHS = 30ULL * 24ULL * 60ULL * 60ULL;

uint64_t get_repo_count(const time_t low, const time_t high,
    const std::string github_token)
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

    char low_date[11];
    char high_date[11];

    std::strftime(low_date, sizeof(low_date), "%Y-%m-%d", std::gmtime(&low));
    std::strftime(high_date, sizeof(high_date), "%Y-%m-%d", std::gmtime(&high));

    const std::string& search_query = std::format("topic:{} created:{}..{}", repo_topic, low_date, high_date);

    httplib::Client cli("https://api.github.com");
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
                { { "search_query", query } },
            },
        }
            .dump(),
        "application/json");
}

std::vector<std::pair<time_t, time_t>> generate_range(const time_t low,
    const time_t high)
{
    if (low > high)
        return { };

    const uint64_t repo_count = get_repo_count(low, high);

    if (repo_count >= 1000) {
        // the binary search formula to find the middle
        // of the array I had learnt during DSA. XD
        const time_t mid = low + (high - low) / 2;

        auto left = generate_range(low, mid);
        const auto& right = generate_range(mid + 1, high);

        left.insert(left.end(), right.begin(), right.end());

        return left;
    }
    return { { low, high } };
}

std::unordered_map<std::string, std::string>
fetch_repositories(const time_t low, const time_t high)
{
    const std::string query = R"""(

            query($search_query: String!, $cursor: String) {
                search(
                    query: $search_query,
                    type: REPOSITORY,
                    first: 100,
                    after: $cursor
                ) {
                    pageInfo {
                        hasNextPage
                        endCursor
                    }

                    nodes {
                        ... on Repository {
                            nameWithOwner
                        }
                    }
                }
            }

    )""";
}

int main()
{
    // time at march 2008 (1 month before GitHub was launched)
    const time_t starting_time = 1204329600;
    const time_t current_time = std::time(nullptr);

    {
        const time_t months_to_process = (current_time - starting_time) / SECONDS_IN_MONTHS;
        std::cout << "Months to process: " << months_to_process << std::endl;
        std::cout << "Years to process: " << months_to_process / 12 << std::endl;
    }

    // I am also thinking if somehow I can take all these ranges
    // afterwards and apply a for loop onto it, and compress these
    // ranges, like, by using 3 parameters low, high and repo_count.
    // and if 2 or more contiguous ranges have repo_count < 1000;
    // I can club those ranges together.
    std::vector<std::pair<time_t, time_t>> ranges;

    for (time_t i = starting_time; i < current_time; i += SECONDS_IN_MONTHS) {
        const time_t low = i;
        const time_t high = i + SECONDS_IN_MONTHS;
        const auto res = generate_range(low, high);
        ranges.insert(ranges.end(), res.begin(), res.end());
    }

    std::unordered_map<std::string, std::string> all_repositories;
    for (const auto& range : ranges) {
        const auto& res = fetch_repositories(range.first, range.second);
        all_repositories.insert(res.begin(), res.end());
    }

    std::cout << current_time << std::endl;

    return 0;
}
