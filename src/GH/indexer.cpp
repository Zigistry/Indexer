#include <iostream>
#include <ctime>

const unsigned long long SECONDS_IN_MONTHS = 30ULL * 24ULL * 60ULL * 60ULL ;




int main() {
    // time at march 2008 (1 month before GitHub was launched)
    const unsigned long long starting_time = 1204329600;
    const unsigned long long current_time = std::time(nullptr);

    {
        unsigned long long int months_to_process = (current_time - starting_time) / SECONDS_IN_MONTHS;
        std::cout << "Months to process: " << months_to_process << std::endl;
        std::cout << "Years to process: " << months_to_process / 12 << std::endl;
    }

    std::cout << current_time << std::endl;

    


    return 0;
}