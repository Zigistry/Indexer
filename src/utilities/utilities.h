#include <iostream>

std::string epoch_to_timestamp_string(const time_t epoch);

struct Range {
    time_t low;
    time_t high;
    uint64_t repo_count;
};