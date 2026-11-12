// I think I should retry doing the indexing part.
// I have just added the docs.excalidraw in the docs.
// thats the logic behind this:
#include <iostream>
#include <random>
struct Range
{
    time_t low;
    time_t high;
    uint64_t repo_count;
};

const time_t month = 30ULL * 24ULL * 60ULL * 60ULL;
const time_t day = 24ULL * 60ULL * 60ULL;

uint64_t get_repo_count(const time_t low, const time_t high)
{
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<int> dist(400, 2500);
    int random_number = dist(gen);
    return random_number;
}

void generate_ranges(std::vector<Range> &ranges, const time_t low, const time_t high)
{
    const uint64_t repo_count = get_repo_count(low, high);
    if (low >= high)
    {
        return;
    }

    if (repo_count < 1000)
    {
        ranges.push_back({low, high, repo_count});
    }
    else
    {
        const time_t mid = low + (high - low) / 2;
        generate_ranges(ranges, low, mid);
        generate_ranges(ranges, mid + 1, high);
    }
}

int main()
{
    const time_t current_epoch = time(NULL);
    const time_t twenty_ten = 1262304000;

    std::vector<Range> res;
    generate_ranges(res, twenty_ten, current_epoch);

    for (const Range r : res)
    {
        std::cout << r.low << " | " << r.high << " | " << r.repo_count << std::endl;
    }

    return 0;
}