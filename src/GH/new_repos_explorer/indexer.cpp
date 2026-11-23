#include <fstream>
#include <iostream>
#include <string>
#include <SQLiteCpp/SQLiteCpp.h>
#include <utilities.h>
#include<vector>

int main()
{
    std::ifstream input_file("./tmp/github_crawler_output.csv");

    SQLite::Database db("./tmp/zigistry.db", SQLite::OPEN_READWRITE);

    SQLite::Statement check_repo_in_repos_query(db, "SELECT 1 FROM repos WHERE id = ? LIMIT 1");
    SQLite::Statement check_owner_in_banned_users_query(db, "SELECT 1 FROM banned_users WHERE id = ? LIMIT 1");


    std::vector<std::string> repos_to_index;

    char line[500];

    while (input_file.getline(line, sizeof(line))) {
        const std::string name_with_owner = line;

        const std::string owner_name = name_with_owner.substr(name_with_owner.find('/') + 1);

        std::cout << owner_name << std::endl;

        std::string id = "gh/" + name_with_owner;

        std::transform(id.begin(), id.end(), id.begin(), ::tolower);
        check_repo_in_repos_query.bind(1, id);

        if (check_repo_in_repos_query.executeStep() || check_owner_in_banned_users_query.executeStep()) {
            continue;
        }

        repos_to_index.push_back(id);
        std::transform(id.begin(), id.end(), id.begin(), ::tolower);
        {
            check_repo_in_repos_query.reset();
            check_owner_in_banned_users_query.reset();
        }
    }

    return 0;
}