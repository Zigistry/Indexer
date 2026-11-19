#include <fstream>
#include <iostream>
#include <sqlite3.h>
#include <utilities.h>

using namespace std;

int main()
{
    std::ifstream input_file("./tmp/github_indexes.csv");
    sqlite3* db;
    if (sqlite3_open("./database/zigistry.db", &db) != SQLITE_OK) {
        std::cerr << "can't open db: " << sqlite3_errmsg(db) << "\n";
        return 1;
    }
    char line[500];

    sqlite3_stmt* stmt;
    const std::string concat = "gh/" + std::string(line);
    const char statement[] = "SELECT * FROM (repos) WHERE id == (?) LIMIT 1";
    sqlite3_prepare_v2(db, statement, -1, &stmt, nullptr);
    sqlite3_bind_text(stmt, 1, concat.c_str(), -1, SQLITE_STATIC);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);

    while (input_file.getline(line, sizeof(line))) {
    }
    return 0;
}
