#include <iostream>

struct Range {
    time_t low;
    time_t high;
    uint64_t repo_count;
};

std::string epoch_to_timestamp_string(const time_t epoch);
time_t timestamp_to_epoch(const std::string& timestamp_string);