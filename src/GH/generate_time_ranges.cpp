#include <generate_time_ranges.hh>

uint64_t get_repo_count(const time_t low, const time_t high, const std::string repo_topic, const std::string github_token, const std::string query_type)
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
        "topic:{} {}:{}..{}", repo_topic, query_type, epoch_to_timestamp_string(low),
        epoch_to_timestamp_string(high));

    uint8_t retry_amount = 0;

    while (retry_amount < 5) {
        thread_local httplib::Client client("https://api.github.com");
        client.set_connection_timeout(10, 0);
        client.set_read_timeout(10, 0);
        client.set_keep_alive(true);
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
            std::cerr << "No response recieved waiting for 50 seconds" << std::endl;
            sleep(50);
            retry_amount++;
            continue;
        }
        if (response->status == 200) {
            const auto responce_as_json = nlohmann::json::parse(response->body);
            if (responce_as_json.contains("errors")) {
                std::cerr
                    << "Responce contained following error wainting for 500 seconds:"
                    << std::endl;
                std::cerr << responce_as_json << std::endl;
                sleep(500);
                retry_amount++;
                continue;
            }

            retry_amount = 0;

            const uint64_t repo_count = responce_as_json["data"]["search"]["repositoryCount"];

            return repo_count;
        } else {
            std::cerr << "Error code: " << response->status << std::endl;
            retry_amount++;

            switch (response->status) {
            case 403:
                std::cerr << "Waiting for 500 seconds." << std::endl;
                sleep(500);
                continue;
            case 503:
            case 504:
            case 500:
            case 502:
                std::cerr << "Waiting for 2 seconds." << std::endl;
                sleep(2);
                continue;
            case 401:
            case 400:
                std::cerr << "Exiting" << std::endl;
                std::exit(1);
            }
        }
    }

    std::cerr << "Exiting after 5 retries." << std::endl;

    std::exit(1);
}


void generate_ranges(std::vector<Range>& ranges, const time_t low,
    const time_t high, const std::string repo_topic, const std::string github_token, const std::string query_type)

{
    if (low > high) {
        return;
    }

    const uint64_t repo_count = get_repo_count(low, high, repo_topic, github_token, query_type);

    if (low == high || repo_count < 1000) {
        #pragma omp critical
        ranges.push_back({ low, high, repo_count });
        std::cerr << "Completed: " << low << "," << high << std::endl;
    } else {
        const time_t mid = low + (high - low) / 2;

        #pragma omp task shared(ranges)
        generate_ranges(ranges, low, mid, repo_topic, github_token);
        #pragma omp task shared(ranges)
        generate_ranges(ranges, mid + 1, high, repo_topic, github_token);
    }
}
