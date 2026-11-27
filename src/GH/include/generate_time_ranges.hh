#include <cstdint>
#include <ctime>
#include <format>
#include <httplib.h>
#include <iostream>
#include <nlohmann/json.hpp>
#include <utilities.h>


void generate_ranges(std::vector<Range>& ranges, const time_t low,
    const time_t high, const std::string repo_topic, const std::string github_token, const std::string query_type);