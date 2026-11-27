#include <nlohmann/json.hpp>
#include <string>

nlohmann::json fetch_and_parse_build_zig_zon(const std::string& owner_name,
    const std::string& repo_name, const std::string& default_branch_name);
