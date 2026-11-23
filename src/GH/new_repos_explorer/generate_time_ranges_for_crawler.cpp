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


#include <generate_time_ranges.hh>

int main(int argc, char* argv[])
{
    if (argc != 2) {
        std::cerr << "Please provide topic name" << std::endl;
        return 1;
    }

    const std::string repo_topic = argv[1];
    const time_t current_epoch = time(nullptr);

    // I was 2 years old back then XD.
    const time_t epoch_2008_march = 1204329600;

    const char* gh_token = getenv("GH_API_KEY");
    if (gh_token == nullptr) {
        std::cerr << "GH_API_KEY not set." << std::endl;
        return 1;
    }
    const std::string github_token = gh_token;

    std::vector<Range> time_ranges;

    // https://stackoverflow.com/questions/68502197/how-do-omp-single-and-omp-task-provide-parallelism
    #pragma omp parallel num_threads(8)
    #pragma omp single
    generate_ranges(time_ranges, epoch_2008_march, current_epoch, repo_topic, github_token, "created");

    std::ofstream output_file("./tmp/github_ranges_for_crawler.csv", std::ios::out);

    for (const Range& time_range : time_ranges) {
        output_file << time_range.low << "," << time_range.high << ","
                    << time_range.repo_count << '\n';
    }

    return 0;
}
