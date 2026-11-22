#include <fstream>
#include <iostream>
#include <string>
#include <SQLiteCpp/SQLiteCpp.h>
#include <utilities.h>

int main()
{
    std::ifstream input_file("./tmp/github_indexes.csv");

    SQLite::Database db("./tmp/zigistry.db", SQLite::OPEN_READWRITE);

    SQLite::Statement query(db, "SELECT 1 FROM repos WHERE id = ? LIMIT 1");

    char line[500];

    while (input_file.getline(line, sizeof(line))) {
        const std::string id = "gh/" + std::string(line);

        query.bind(1, id);

        if (!query.executeStep()) {
            std::cout << id << " not in db" << std::endl;
            SQLite::Statement query2(db, "INSERT INTO repo_pipeline_queue (?,?,?,?,?,?)");
            query2.bind(1, id);
            query2.bind(1, "pending_check");

        }
        query.reset();
    }

    return 0;
}