#include <iostream>
#include <fstream>
#include <utilities.h>

using namespace std;

int main()
{
    std::vector<std::string> list_of_repos;
    std::ifstream input_file("./tmp/github_indexes.csv");

    char line[500];
    while (input_file.getline(line, sizeof(line))) {
        list_of_repos.push_back(line);
    }
    return 0;
}