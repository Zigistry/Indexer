#include <generate_time_ranges.hh>

int main(int argc, char* argv[])
{
    if (argc != 2) {
        std::cerr << "Please provide topic name" << std::endl;
        return 1;
    }

    const std::string repo_topic = argv[1];
    const time_t current_epoch = time(nullptr);

    std::ifstream previous_index_timestamp_file("./tmp/last_updated.txt");
    char line[100];

    previous_index_timestamp_file.getline(line, sizeof(line));

    const uint64_t the_time_from_which_we_need_to_start_indexing = std::stoull(line);

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
    generate_ranges(time_ranges, the_time_from_which_we_need_to_start_indexing, current_epoch, repo_topic, github_token, "updated");

    std::ofstream output_file("./tmp/github_ranges_for_updater.csv", std::ios::out);

    for (const Range& time_range : time_ranges) {
        output_file << time_range.low << "," << time_range.high << ","
                    << time_range.repo_count << '\n';
    }

    return 0;
}
