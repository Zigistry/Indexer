#include <iostream>
#include <ctime>
#include <cstddef>
#include <cstdint>


// Using python as topic because
// it has been around since 2008
// and is a very common topic.
// best to stress test this.
const std::string repo_topic = "python";
const uint64_t SECONDS_IN_MONTHS = 30ULL * 24ULL * 60ULL * 60ULL;



void process_range(const uint64_t low, const uint64_t high) {
    if(low <= high) return;
    
    const uint8_t repo_count = get_repo_count(low, high);


    if(repo_count > 1000) {
        // the binary search formula to find the middle
        // of the array I had learnt during DSA. XD
        const uint64_t mid = low + (high - low) / 2;
        process_range(low, mid);
        process_range(mid + 1, high);
        return;
    }



}


int main() {
    // time at march 2008 (1 month before GitHub was launched)
    const uint64_t starting_time = 1204329600;
    const uint64_t current_time = std::time(nullptr);

    {
        const uint64_t months_to_process = (current_time - starting_time) / SECONDS_IN_MONTHS;
        std::cout << "Months to process: " << months_to_process << std::endl;
        std::cout << "Years to process: " << months_to_process / 12 << std::endl;
    }


    for(uint64_t i = starting_time; i < current_time; i += SECONDS_IN_MONTHS) {
        uint64_t low = i;
        uint64_t high = i + SECONDS_IN_MONTHS;
        process_range(low, high);
    }


    std::cout << current_time << std::endl;

    


    return 0;
}