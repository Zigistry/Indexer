#include <iostream>
#include <ctime>
#include <cstddef>
#include <cstdint>

// Using python as topic because
// it has been around since 2008
// and is a very common topic.
// best to stress test this.
const std::string repo_topic = "python";
const uint64_t SECONDS_IN_MONTHS = 30ULL * 24ULL * 60ULL * 60ULL;


uint64_t get_repo_count(uint64_t low, uint64_t high) {
    const std::string query = R"""(

            query($search_query: String!, $cursor: String) {
                search(
                    query: $search_query,
                    type: REPOSITORY,
                    first: 100,
                    after: $cursor
                ) {
                    repositoryCount
                }
            }

    )""";
}

 

std::vector<std::pair<uint64_t, uint64_t>> generate_range(const uint64_t low, const uint64_t high) {
    if(low > high) return {};
    
    const uint64_t repo_count = get_repo_count(low, high);

    if(repo_count >= 1000) {
        // the binary search formula to find the middle
        // of the array I had learnt during DSA. XD
        const uint64_t mid = low + (high - low) / 2;

        auto left = generate_range(low, mid);
        const auto &right = generate_range(mid + 1, high);

        left.insert(left.end(), right.begin(), right.end());

        return left;
    }
    return {{low, high}};
}


std::unordered_map<std::string, std::string> fetch_repositories(const uint64_t low, const uint64_t high) {
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


int main() {
    // time at march 2008 (1 month before GitHub was launched)
    const uint64_t starting_time = 1204329600;
    const uint64_t current_time = std::time(nullptr);

    {
        const uint64_t months_to_process = (current_time - starting_time) / SECONDS_IN_MONTHS;
        std::cout << "Months to process: " << months_to_process << std::endl;
        std::cout << "Years to process: " << months_to_process / 12 << std::endl;
    }


    // I am also thinking if somehow I can take all these ranges
    // afterwards and apply a for loop onto it, and compress these
    // ranges, like, by using 3 parameters low, high and repo_count.
    // and if 2 or more contiguous ranges have repo_count < 1000;
    // I can club those ranges together.
    std::vector<std::pair<uint64_t, uint64_t>> ranges;

    for(uint64_t i = starting_time; i < current_time; i += SECONDS_IN_MONTHS) {
        uint64_t low = i;
        uint64_t high = i + SECONDS_IN_MONTHS;
        auto res = generate_range(low, high);
        ranges.insert(ranges.end(), res.begin(), res.end());
    }


    std::unordered_map<std::string, std::string> all_repositories;
    for(const auto &range: ranges) {
        const auto &res = fetch_repositories(range.first, range.second);
        all_repositories.insert(res.begin(), res.end());
    }

    std::cout << current_time << std::endl;


    return 0;
}